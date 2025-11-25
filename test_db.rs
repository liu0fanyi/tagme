use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    // 获取应用数据目录
    let mut db_path = PathBuf::from(env!("USERPROFILE"));
    db_path.push("AppData");
    db_path.push("Roaming");
    db_path.push("com.tagme.app");
    db_path.push("tagme_app.db");

    println!("数据库路径: {:?}", db_path);

    if !db_path.exists() {
        println!("数据库文件不存在！");
        return Ok(());
    }

    let conn = Connection::open(&db_path)?;

    // 查看现有tags
    println!("=== 现有的Tags ===");
    let mut stmt = conn.prepare("SELECT id, name, parent_id, color FROM tags")?;
    let tag_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    let mut count = 0;
    for tag in tag_iter {
        if let Ok((id, name, parent_id, color)) = tag {
            println!("ID: {}, Name: {}, Parent: {:?}, Color: {:?}", id, name, parent_id, color);
            count += 1;
        }
    }

    if count == 0 {
        println!("没有找到任何tag，正在创建测试数据...");

        // 创建一些测试tag
        let test_tags = vec![
            ("工作", None, Some("#FF6B6B")),
            ("个人", None, Some("#4ECDC4")),
            ("重要", None, Some("#45B7D1")),
            ("项目A", Some(1), Some("#96CEB4")),
            ("项目B", Some(1), Some("#FECA57")),
        ];

        for (name, parent_id, color) in test_tags {
            conn.execute(
                "INSERT INTO tags (name, parent_id, color, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![name, parent_id, color, 1638360000],
            )?;
            println!("已创建tag: {}", name);
        }

        println!("测试数据创建完成！");
    }

    Ok(())
}