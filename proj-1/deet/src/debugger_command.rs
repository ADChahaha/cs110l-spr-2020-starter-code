use crate::dwarf_data::{DwarfData, Error as DwarfError};
pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Continue,
    BackTrace,
    Break(usize),
}

fn parse_address(addr: &str) -> Option<usize> {
    let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
        &addr[2..]
    } else {
        &addr
    };
    usize::from_str_radix(addr_without_0x, 16).ok()
}

fn parse_line(line: &str) -> Option<usize> {
    usize::from_str_radix(line, 10).ok()
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>, debug_data: &DwarfData) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            }
            "c" | "cont" | "continue" => Some(DebuggerCommand::Continue),
            "bt" | "backtrace" => Some(DebuggerCommand::BackTrace),
            "b" | "break" => {
                if tokens.len() != 2 {
                    None
                } else {
                    let str = tokens[1];
                    // addr
                    if str.starts_with("*") {
                        Some(DebuggerCommand::Break(parse_address(str)?))
                    }
                    // line number
                    else if let Some(line) = parse_line(str) {
                        Some(DebuggerCommand::Break(
                            debug_data.get_addr_for_line(None, line)?,
                        ))
                    }
                    // function
                    else {
                        Some(DebuggerCommand::Break(
                            debug_data.get_addr_for_function(None, str)?,
                        ))
                    }
                }
            }

            // Default case:
            _ => None,
        }
    }
}
