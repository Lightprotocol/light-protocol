/**
 * This code is take from circom/parser because preprocess is not public
 */
use program_structure::{error_code::ReportCode, file_definition::FileLocation};
use program_structure::{error_definition::Report, file_definition::FileID};

fn match_entrypoint<I>(chars: &mut I) -> bool
where
    I: Iterator<Item = char> + Clone + std::fmt::Debug,
{
    let target = "[entrypoint]".chars().collect::<Vec<_>>();
    let local_chars = chars.clone();
    for (c, lc) in target.iter().zip(local_chars){
        if *c != lc {
            return false;
        }
    }
    true
}

pub fn preprocess(expr: &str, file_id: FileID, filter_macros: bool) -> Result<String, Report> {
    let mut pp = String::new();
    let mut state = 0;
    let mut loc = 0;
    let mut block_start = 0;

    let mut it = expr.chars();
    while let Some(c0) = it.next() {
        loc += 1;
        match (state, c0) {
            (0,'#') => {
                if !filter_macros {
                    pp.push(c0);

                } else {
                    let res = match_entrypoint(&mut it);
                    if !res {
                        pp.push(c0);
                    } else {
                        for _ in 0.."[entrypoint]".len() {
                            it.next();
                        }
                    }
                }
                
            }
            (0, '/') => {
                loc += 1;
                match it.next() {
                    
                    Some('/') => {
                        state = 1;
                        pp.push(' ');
                        pp.push(' ');
                    }
                    Some('*') => {
                        block_start = loc;
                        state = 2;
                        pp.push(' ');
                        pp.push(' ');
                    }
                    Some(c1) => {
                        pp.push(c0);
                        pp.push(c1);
                    }
                    None => {
                        pp.push(c0);
                        break;
                    }
                }
            }
            (0, _) => pp.push(c0),
            (1, '\n') => {
                pp.push(c0);
                state = 0;
            }
            (2, '*') => {
                loc += 1;
                let mut next = it.next();
                while next == Some('*') {
                    pp.push(' ');
                    loc += 1;
                    next = it.next();
                }
                match next {
                    Some('/') => {
                        pp.push(' ');
                        pp.push(' ');
                        state = 0;
                    }
                    Some(c) => {
                        pp.push(' ');
                        for _i in 0..c.len_utf8() {
                            pp.push(' ');
                        }
                    }
                    None => {}
                }
            }
            (_, c) => {
                for _i in 0..c.len_utf8() {
                    pp.push(' ');
                }
            }
        }
    }
    if state == 2 {
        let error = UnclosedCommentError {
            location: block_start..block_start,
            file_id,
        };
        Err(UnclosedCommentError::produce_report(error))
    } else {
        Ok(pp)
    }
}
pub struct UnclosedCommentError {
    pub location: FileLocation,
    pub file_id: FileID,
}

impl UnclosedCommentError {
    pub fn produce_report(error: Self) -> Report {
        let mut report = Report::error(format!("unterminated /* */"), ReportCode::ParseFail);
        report.add_primary(
            error.location,
            error.file_id,
            format!("Comment starts here"),
        );
        report
    }
}
