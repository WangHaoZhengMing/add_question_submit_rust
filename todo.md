   Err(e) => {
                error!(
                    "[试卷 {}] 题目 {} 处理失败: {}",
                    paper_index, question_index, e
                );
                stats.skipped += 1;
            }
        }
也要写入文件中
