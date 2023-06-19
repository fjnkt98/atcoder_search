use crate::{modules::users::scraper::RankingPageScraper, types::tables::User};
use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::Url;
use sqlx::{self, postgres::Postgres, Pool};
use tokio::time::{self, Duration};

static SCRAPER: Lazy<RankingPageScraper> = Lazy::new(|| RankingPageScraper::new());

pub struct UserCrawler<'a> {
    url: Url,
    pool: &'a Pool<Postgres>,
    client: Client,
}

impl<'a> UserCrawler<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        UserCrawler {
            url: Url::parse("https://atcoder.jp/ranking").unwrap(),
            pool,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    /// ランキングページのうちの1ページを取得してユーザ一覧を取得するメソッド
    pub async fn fetch_page(&self, index: usize) -> Result<Vec<User>> {
        let res = self
            .client
            .get(self.url.clone())
            .query(&[
                ("contestType", "algo"),
                ("page", index.to_string().as_ref()),
            ])
            .send()
            .await?;

        match res.error_for_status_ref() {
            Ok(_) => {}
            Err(e) => {
                let message = format!(
                    "error response returned from AtCoder user ranking page: {:?}",
                    e
                );
                tracing::error!(message);
                anyhow::bail!(message)
            }
        };

        let html = res.text().await?;

        SCRAPER.extract_user_digests(&html).ok_or(anyhow::anyhow!(
            "failed to extract user information from ranking page at {}",
            index
        ))
    }

    pub async fn save(&self, users: &Vec<User>) -> Result<()> {
        let first = users
            .first()
            .and_then(|first| Some(first.rank))
            .unwrap_or(0);
        let last = users.last().and_then(|last| Some(last.rank)).unwrap_or(0);
        tracing::info!("Start to save user information from {} to {}.", first, last);

        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                let message = format!("failed to start transaction cause: {:?}", e);
                tracing::error!(message);
                anyhow::bail!(message)
            }
        };

        for user in users.iter() {
            let result = sqlx::query(r#"
                MERGE INTO "users"
                USING
                    (VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)) AS "user"("user_name", "rating", "highest_rating", "affiliation", "birth_year", "country", "join_count", "rank", "wins")
                ON
                    "users"."user_name" = "user"."user_name"
                WHEN MATCHED THEN
                    UPDATE SET (
                        "rating",
                        "highest_rating",
                        "affiliation",
                        "birth_year",
                        "country",
                        "join_count",
                        "rank",
                        "wins"
                    ) = (
                        "user"."rating",
                        "user"."highest_rating",
                        "user"."affiliation",
                        "user"."birth_year",
                        "user"."country",
                        "user"."join_count",
                        "user"."rank",
                        "user"."wins"
                    )
                WHEN NOT MATCHED THEN
                    INSERT (
                        "user_name",
                        "rating",
                        "highest_rating",
                        "affiliation",
                        "birth_year",
                        "country",
                        "join_count",
                        "rank",
                        "wins"
                    )
                    VALUES (
                        "user"."user_name",
                        "user"."rating",
                        "user"."highest_rating",
                        "user"."affiliation",
                        "user"."birth_year",
                        "user"."country",
                        "user"."join_count",
                        "user"."rank",
                        "user"."wins"
                    );
                "#)
                .bind(&user.user_name)
                .bind(&user.rating)
                .bind(&user.highest_rating)
                .bind(&user.affiliation)
                .bind(&user.birth_year)
                .bind(&user.country)
                .bind(&user.crown)
                .bind(&user.join_count)
                .bind(&user.rank)
                .bind(&user.wins)
                .execute(&mut tx)
                .await;

            // エラーが発生したらトランザクションをロールバックしてエラーを早期リターンする
            if let Err(e) = result {
                let message = format!("an error occurred: {:?}, at saving {:?}", e, user);
                tracing::error!(message);
                tx.rollback().await?;

                anyhow::bail!(message);
            }
        }

        tracing::info!("Users from {} to {} successfully saved.", first, last);

        Ok(())
    }

    pub async fn crawl(&self) -> Result<()> {
        tracing::info!("Start to crawl active user information");

        let mut i = 1;
        while let Ok(users) = self.fetch_page(i).await {
            tracing::info!("Crawl ranking page {}", i);
            self.save(&users).await?;

            time::sleep(Duration::from_secs(1)).await;
            i += 1;
        }

        Ok(())
    }
}
