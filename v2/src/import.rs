use std::collections::HashMap;

use rusqlite::{Result, Row, Statement, ToSql, Transaction, NO_PARAMS};

use crate::structs::*;

pub fn get_wpp(row: &Row, names: &HashMap<String, usize>) -> Result<WallpaperPath> {
    let path = row.get(names["path"])?;
    let sha1 = row.get(names["sha1"])?;
    Ok(WallpaperPath { sha1, path })
}

pub fn get_wpi(row: &Row, names: &HashMap<String, usize>) -> Result<WallpaperInfo> {
    let nsfw: Option<i32> = row.get(names["nsfw"])?;
    let purity = match nsfw {
        Some(0) => Purity::Sketchy,
        Some(1) => Purity::NSFW,
        _ => Purity::Pure,
    };
    let vote: Option<i32> = row.get(names["vote"])?;
    let fav: Option<i32> = row.get(names["fav"])?;
    let deleted_option: Option<i32> = row.get(names["deleted"])?;
    let deleted = deleted_option.map(|_| true).unwrap_or(false);

    let collection = if deleted {
        Collection::Trash
    } else if vote.unwrap_or(0) > 0 {
        Collection::Display
    } else if vote.unwrap_or(0) < 0 {
        Collection::Shelf
    } else if fav.map(|_| true).unwrap_or(false) {
        Collection::Favorite
    } else {
        Collection::Normal
    };

    Ok(WallpaperInfo {
        sha1: row.get(names["sha1"])?,
        purity,
        collection,
    })
}

fn query_helper(stmt: &Statement) -> Result<HashMap<String, usize>> {
    let names = stmt.column_names();
    let mut map = HashMap::new();
    for name in names {
        map.insert(String::from(name), stmt.column_index(name)?);
    }
    Ok(map)
}

pub fn import(tx: &Transaction) -> Result<()> {


    {
        let mut query_stmt = tx.prepare(
            "select path, sha1, vote, fav, deleted, cast (nsfw as INTEGER) AS nsfw from wallpaper",
        )?;

        let names = query_helper(&query_stmt)?;

        for key in names.keys() {
            println!("{}: {}", key, names[key]);
        }

        let rows = query_stmt.query_map(NO_PARAMS, |row| {
            let wpp = get_wpp(row, &names);
            let wpi = get_wpi(row, &names);
            Ok((wpp, wpi))
        })?;

        let mut file_stmt = tx.prepare("insert into files (sha1, path) values (?, ?)")?;

        let mut info_stmt =
            tx.prepare("insert or fail into info (sha1, collection, purity) values (?, ?, ?)")?;

        for row in rows {
            let (wpp, wpir) = row?;
            match wpir {
                Ok(wpi) => {
                    let sha = wpi.sha1;
                    let col = wpi.collection;
                    let pur = wpi.purity;

                    info_stmt.execute::<&[&dyn ToSql]>(&[&sha, &col, &pur])?;
                }
                Err(e) => {
                    println!("err wpp {}", e)
                }
            }

            match wpp {
                Err(e) => {
                    println!("err wpp {}", e)
                }
                Ok(wppo) => {
                    file_stmt.execute::<&[&dyn ToSql]>(&[&wppo.sha1, &wppo.path])?;
                }
            }
        }
        // Command::new("set-wallpaper")
        //     .arg(full)
        //     .spawn()
        //     .expect("failed to execute process");
    };

    Ok(())
}
