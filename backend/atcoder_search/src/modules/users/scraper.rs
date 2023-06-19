use std::path::Path;

use crate::types::tables::User;
use scraper::{ElementRef, Html, Selector};

pub struct RankingPageScraper {
    table: Selector,
    tr: Selector,
    td: Selector,
    td_a: Selector,
    td_img: Selector,
    a_img: Selector,
    a_span: Selector,
}

impl RankingPageScraper {
    pub fn new() -> Self {
        let table = Selector::parse(".table > tbody").unwrap();
        let tr = Selector::parse("tr").unwrap();
        let td = Selector::parse("td").unwrap();
        let td_a = Selector::parse("td > a").unwrap();
        let td_img = Selector::parse("td > img").unwrap();
        let a_img = Selector::parse("a > img").unwrap();
        let a_span = Selector::parse("a > span").unwrap();

        Self {
            table,
            tr,
            td,
            td_a,
            td_img,
            a_img,
            a_span,
        }
    }

    pub fn extract_user_digests(&self, html: &str) -> Option<Vec<User>> {
        let html = Html::parse_document(html);

        let table = match html.select(&self.table).next() {
            Some(table) => table,
            None => {
                tracing::warn!("failed to extract user ranking table from page html");
                return None;
            }
        };

        let mut users: Vec<User> = Vec::with_capacity(100);

        for (i, tr) in table.select(&self.tr).enumerate() {
            let td: Vec<ElementRef<'_>> = tr.select(&self.td).collect();

            let rank: i32 = td
                .get(0)
                .and_then(|elem| elem.text().next())
                .and_then(|text| text.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    tracing::warn!("failed to extract user rank at {}", i);
                    -1
                });
            let (country, user_name, affiliation, crown) = td
                .get(1)
                .and_then(|td_1| {
                    let a: Vec<ElementRef<'_>> = td_1.select(&self.td_a).collect();
                    let country = a
                        .get(0)
                        .and_then(|a| a.select(&self.a_img).next())
                        .and_then(|img| img.value().attr("src"))
                        .and_then(|src| Path::new(src).file_stem())
                        .and_then(|stem| stem.to_str())
                        .and_then(|country| Some(country.to_string()));
                    let user_name = a
                        .get(1)
                        .and_then(|a| a.select(&self.a_span).next())
                        .and_then(|span| span.text().next())
                        .and_then(|text| Some(text.to_string()))
                        .unwrap_or_else(|| {
                            tracing::warn!("failed to extract user name at {}", i);
                            String::default()
                        });
                    let affiliation = a
                        .get(2)
                        .and_then(|a| a.select(&self.a_span).next())
                        .and_then(|span| span.text().next())
                        .and_then(|text| Some(text.to_string()));
                    let crown = td_1
                        .select(&self.td_img)
                        .next()
                        .and_then(|img| img.value().attr("src"))
                        .and_then(|src| Path::new(src).file_stem())
                        .and_then(|stem| stem.to_str())
                        .and_then(|crown| Some(crown.to_string()));

                    Some((country, user_name, affiliation, crown))
                })
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "failed to extract user's country, name, affiliation, and crown at {}.",
                        i
                    );
                    (None, String::default(), None, None)
                });
            let birth_year = td
                .get(2)
                .and_then(|elem| elem.text().next())
                .and_then(|text| text.parse::<i32>().ok());
            let rating = td
                .get(3)
                .and_then(|elem| elem.text().next())
                .and_then(|text| text.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    tracing::warn!("failed to extract the rating at {}.", i);
                    -1
                });
            let highest_rating = td
                .get(4)
                .and_then(|elem| elem.text().next())
                .and_then(|text| text.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    tracing::warn!("failed to extract the highest rating at {}.", i);
                    -1
                });
            let join_count = td
                .get(5)
                .and_then(|elem| elem.text().next())
                .and_then(|text| text.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    tracing::warn!("failed to extract join count at {}.", i);
                    -1
                });
            let wins = td
                .get(6)
                .and_then(|elem| elem.text().next())
                .and_then(|text| text.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    tracing::warn!("failed to extract wins at {}.", i);
                    -1
                });

            users.push(User {
                affiliation,
                birth_year,
                country,
                crown,
                highest_rating,
                join_count,
                rank,
                rating,
                user_name,
                wins,
            })
        }

        Some(users)
    }
}
