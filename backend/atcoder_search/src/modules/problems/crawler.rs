use crate::types::{
    contest::ContestJson,
    problem::{ProblemDifficulty, ProblemJson},
    tables::Contest,
};
use anyhow::{Context, Result};
use minify_html::{minify, Cfg};
use reqwest::Client;
use reqwest::Url;
use sqlx::{
    self,
    postgres::{PgRow, Postgres},
    Pool, Row,
};
use std::collections::{HashMap, HashSet};
use tokio::time::{self, Duration};

pub struct ContestCrawler<'a> {
    url: Url,
    pool: &'a Pool<Postgres>,
    client: Client,
}

impl<'a> ContestCrawler<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        ContestCrawler {
            url: Url::parse("https://kenkoooo.com/atcoder/resources/contests.json").unwrap(),
            pool,
            client: Client::builder()
                .gzip(true)
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    /// AtCoderProblemsからコンテスト情報を取得するメソッド
    pub async fn fetch_contest_list(&self) -> Result<Vec<ContestJson>> {
        tracing::info!("Start to retrieve contests information from AtCoder Problems");
        let res = self.client.get(self.url.clone()).send().await?;
        let contests: Vec<ContestJson> = res.json().await?;

        tracing::info!(
            "{} contests information successfully retrieved.",
            contests.len()
        );

        Ok(contests)
    }

    /// AtCoderProblemsから取得したコンテスト情報からデータベースへ格納する用のモデルを作って返すメソッド
    pub async fn crawl(&self) -> Result<Vec<Contest>> {
        tracing::info!("Start to crawl contests information.");
        let contests: Vec<Contest> = self
            .fetch_contest_list()
            .await?
            .iter()
            .map(|contest| Contest {
                contest_id: contest.id.clone(),
                start_epoch_second: contest.start_epoch_second.clone(),
                duration_second: contest.duration_second.clone(),
                title: contest.title.clone(),
                rate_change: contest.rate_change.clone(),
                category: contest.categorize(),
            })
            .collect();
        tracing::info!(
            "{} contests information successfully crawled.",
            contests.len()
        );

        Ok(contests)
    }

    /// コンテスト情報をデータベースへ保存するメソッド
    ///
    /// データの保存にMERGE INTO文(PostgreSQL 15から)を使用している
    /// コンテスト情報の存在判定にIDを使用し、IDが存在すればUPDATE、IDが存在しなければINSERTを実行する
    /// UPDATE時はすべての情報をUPDATEするようにしている
    pub async fn save(&self, contests: &Vec<Contest>) -> Result<()> {
        tracing::info!("Start to save contests information.");
        // トランザクション開始
        let mut tx = self.pool.begin().await.with_context(|| {
            let message = "failed to start transaction";
            tracing::error!(message);
            message
        })?;

        // 各コンテスト情報を一つずつ処理する
        for contest in contests.iter() {
            let result = sqlx::query("
                MERGE INTO contests
                USING
                    (VALUES($1, $2, $3, $4, $5, $6)) AS contest(contest_id, start_epoch_second, duration_second, title, rate_change, category)
                ON
                    contests.contest_id = contest.contest_id
                WHEN MATCHED THEN
                    UPDATE SET (contest_id, start_epoch_second, duration_second, title, rate_change, category) = (contest.contest_id, contest.start_epoch_second, contest.duration_second, contest.title, contest.rate_change, contest.category)
                WHEN NOT MATCHED THEN
                    INSERT (contest_id, start_epoch_second, duration_second, title, rate_change, category)
                    VALUES (contest.contest_id, contest.start_epoch_second, contest.duration_second, contest.title, contest.rate_change, contest.category);
                ")
                .bind(&contest.contest_id)
                .bind(&contest.start_epoch_second)
                .bind(&contest.duration_second)
                .bind(&contest.title)
                .bind(&contest.rate_change)
                .bind(&contest.category)
                .execute(&mut tx)
                .await;

            // エラーが発生したらトランザクションをロールバックしてエラーを早期リターンする
            if let Err(e) = result {
                tracing::error!("an error occurred at saving {:?}.", contest);
                tx.rollback().await?;
                anyhow::bail!("an error occurred in transaction: {}", e);
            }
        }

        tx.commit().await?;
        tracing::info!("{} contests successfully saved.", contests.len());

        Ok(())
    }

    /// コンテスト情報の取得からデータベースへの保存までの一連の処理を行うメソッド
    pub async fn run(&self) -> Result<()> {
        let contests = self.crawl().await?;
        self.save(&contests).await?;

        Ok(())
    }
}
pub struct ProblemCrawler<'a> {
    url: Url,
    pool: &'a Pool<Postgres>,
    client: Client,
}

impl<'a> ProblemCrawler<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        ProblemCrawler {
            url: Url::parse("https://kenkoooo.com/atcoder/resources/problems.json").unwrap(),
            pool: pool,
            client: Client::builder()
                .gzip(true)
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    /// AtCoder Problemsから問題情報の一覧を取得するメソッド
    pub async fn fetch_problem_list(&self) -> Result<Vec<ProblemJson>> {
        tracing::info!("Attempting to get problem list from AtCoder Problems...");
        let res = self.client.get(self.url.clone()).send().await?;
        let problems: Vec<ProblemJson> = res.json().await?;

        tracing::info!("{} problems collected.", problems.len());

        Ok(problems)
    }

    /// 問題ページをクロールしてHTML情報を取得するメソッド
    ///
    /// クロール間隔は300msにしてある。
    ///
    /// - target: クロール対象の問題のリスト
    pub async fn crawl(&self, url: &str, config: &Cfg) -> Result<String> {
        tracing::info!("Crawl {}", url);
        let res = self.client.get(url).send().await?;
        let body = res.bytes().await?;
        let html = String::from_utf8(minify(&body, &config))?;

        Ok(html)
    }

    /// AtCoder Problemsから得た一覧情報とデータベースにある情報を比較し、
    /// 未取得の問題を検出するメソッド
    pub async fn detect_diff(&self) -> Result<Vec<ProblemJson>> {
        let exists_problems: HashSet<String> = HashSet::from_iter(
            sqlx::query(
                r#"
            SELECT problem_id FROM problems;
            "#,
            )
            .map(|row: PgRow| row.get(0))
            .fetch_all(self.pool)
            .await?
            .iter()
            .cloned(),
        );

        let target: Vec<ProblemJson> = self
            .fetch_problem_list()
            .await?
            .into_iter()
            .filter(|problem| !exists_problems.contains(&problem.id))
            .collect();

        tracing::info!("{} problems are now target for collection.", target.len());

        Ok(target)
    }

    /// 問題データをデータベースに格納するメソッド
    pub async fn save(&self, targets: &Vec<ProblemJson>, duration: Duration) -> Result<()> {
        let config = Cfg {
            do_not_minify_doctype: true,
            ensure_spec_compliant_unquoted_attribute_values: false,
            keep_closing_tags: true,
            keep_html_and_head_opening_tags: false,
            keep_spaces_between_attributes: false,
            keep_comments: false,
            minify_css: true,
            minify_js: true,
            remove_bangs: false,
            remove_processing_instructions: false,
            minify_css_level_1: true,
            minify_css_level_2: false,
            minify_css_level_3: false,
        };

        for problem in targets.iter() {
            let mut tx = self.pool.begin().await?;

            let url = format!(
                "https://atcoder.jp/contests/{}/tasks/{}",
                problem.contest_id, problem.id
            );
            let html = self.crawl(&url, &config).await?;

            let result = sqlx::query(r"
                MERGE INTO problems
                USING
                    (VALUES($1, $2, $3, $4, $5, $6, $7)) AS problem(problem_id, contest_id, problem_index, name, title, url, html)
                ON
                    problems.problem_id = problem.problem_id
                WHEN MATCHED THEN
                    UPDATE SET (problem_id, contest_id, problem_index, name, title, url, html) = (problem.problem_id, problem.contest_id, problem.problem_index, problem.name, problem.title, problem.url, problem.html)
                WHEN NOT MATCHED THEN
                    INSERT (problem_id, contest_id, problem_index, name, title, url, html)
                    VALUES (problem.problem_id, problem.contest_id, problem.problem_index, problem.name, problem.title, problem.url, problem.html);
                ")
                .bind(&problem.id)
                .bind(&problem.contest_id)
                .bind(&problem.problem_index)
                .bind(&problem.name)
                .bind(&problem.title)
                .bind(&url)
                .bind(html)
                .execute(&mut tx)
                .await;

            match result {
                Ok(_) => {
                    tracing::info!("Problem {} was saved.", problem.id);
                    tx.commit().await?;
                }
                Err(e) => {
                    tracing::error!("An error occurred at {:?}: {}", problem.id, e);
                    tx.rollback().await?;
                    anyhow::bail!("an error occurred: {}", e);
                }
            }

            time::sleep(duration).await;
        }

        Ok(())
    }

    /// 問題情報の取得からデータベースへの保存までの一連の処理を行うメソッド
    ///
    /// - allがtrueのときはすべての問題を対象にクロールを行う
    /// - allがfalseのときは差分取得のみを行う
    pub async fn run(&self, all: bool, duration: Duration) -> Result<()> {
        let targets = if all {
            self.fetch_problem_list().await?
        } else {
            self.detect_diff().await?
        };

        self.save(&targets, duration).await?;

        Ok(())
    }
}

pub struct DifficultyCrawler<'a> {
    url: Url,
    pool: &'a Pool<Postgres>,
    client: Client,
}

impl<'a> DifficultyCrawler<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        Self {
            url: Url::parse("https://kenkoooo.com/atcoder/resources/problem-models.json").unwrap(),
            pool,
            client: Client::builder()
                .gzip(true)
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// 問題の難易度情報を取得してハッシュマップとして返すメソッド
    async fn fetch_difficulties(&self) -> Result<HashMap<String, ProblemDifficulty>> {
        tracing::info!("Attempting to get difficulties from AtCoder Problems...");
        let res = self.client.get(self.url.clone()).send().await?;
        let difficulties: HashMap<String, ProblemDifficulty> = res.json().await?;

        Ok(difficulties)
    }

    pub async fn save(&self, difficulties: &HashMap<String, ProblemDifficulty>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for (problem_id, difficulty) in difficulties.iter() {
            let result = sqlx::query(
                r#"
                MERGE INTO "difficulties"
                USING
                    (
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ) AS "difficulty"(
                        "problem_id", "slope", "intercept", "variance", "difficulty", "discrimination", "irt_loglikelihood", "irt_users", "is_experimental"
                    )
                ON
                    "difficulties"."problem_id" = "difficulty"."problem_id"
                WHEN MATCHED THEN
                    UPDATE SET (
                        "problem_id", "slope", "intercept", "variance", "difficulty", "discrimination", "irt_loglikelihood", "irt_users", "is_experimental"
                    ) = (
                        "difficulty"."problem_id",
                        "difficulty"."slope",
                        "difficulty"."intercept",
                        "difficulty"."variance",
                        "difficulty"."difficulty",
                        "difficulty"."discrimination",
                        "difficulty"."irt_loglikelihood",
                        "difficulty"."irt_users",
                        "difficulty"."is_experimental"
                    )
                WHEN NOT MATCHED THEN
                    INSERT (
                        "problem_id", "slope", "intercept", "variance", "difficulty", "discrimination", "irt_loglikelihood", "irt_users", "is_experimental"
                    )
                    VALUES (
                        "difficulty"."problem_id",
                        "difficulty"."slope",
                        "difficulty"."intercept",
                        "difficulty"."variance",
                        "difficulty"."difficulty",
                        "difficulty"."discrimination",
                        "difficulty"."irt_loglikelihood",
                        "difficulty"."irt_users",
                        "difficulty"."is_experimental"
                    );
            "#,
            )
            .bind(&problem_id)
            .bind(difficulty.slope)
            .bind(difficulty.intercept)
            .bind(difficulty.variance)
            .bind(difficulty.difficulty)
            .bind(difficulty.discrimination)
            .bind(difficulty.irt_loglikelihood)
            .bind(difficulty.irt_users)
            .bind(difficulty.is_experimental)
            .execute(&mut tx)
            .await;

            if let Err(e) = result {
                let message = format!("an error occurred at saving {}: [{:?}]", problem_id, e);
                tracing::error!(message);
                tx.rollback().await?;
                anyhow::bail!(message);
            }
        }

        tx.commit().await?;
        tracing::info!("{} difficulties successfully saved.", difficulties.len());

        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let difficulties = self.fetch_difficulties().await?;
        self.save(&difficulties).await?;

        Ok(())
    }
}
