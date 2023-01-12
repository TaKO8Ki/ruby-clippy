use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use lib_ruby_parser::{
    nodes::{Begin, Lvar, Lvasgn, Send},
    source::DecodedInput,
    Loc, Node, Parser, ParserOptions, ParserResult,
};
use tracing::debug;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mut builder = TypesBuilder::new();
    builder.add_defaults();
    builder.select("ruby");
    let matcher = builder.build().unwrap();

    for entry in WalkBuilder::new(".")
        .types(matcher)
        .build()
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry
            .file_type()
            .map_or(false, |file_type| file_type.is_dir())
        {
            continue;
        }
        let f_name = String::from(entry.file_name().to_string_lossy());
        debug!("Parsing {}", f_name);
        if entry
            .file_type()
            .map_or(false, |file_type| file_type.is_dir())
        {
            continue;
        }
        debug!("Parsing {:?}", entry.path());
        let Some(text) = std::fs::read_to_string(entry.path()).ok() else {
            continue
        };
        let options = ParserOptions {
            buffer_name: f_name,
            ..Default::default()
        };
        let parser = Parser::new(text.as_bytes(), options);

        if let ParserResult {
            ast: Some(ast),
            input,
            ..
        } = parser.do_parse()
        {
            debug!("Parsing");
            ambiguous_assignment(*ast, &input);
        }
    }

    Ok(())
}

fn ambiguous_assignment(node: Node, input: &DecodedInput) {
    // println!("{:#?}", node);
    if let Node::Begin(Begin { statements, .. }) = node {
        for statement in statements {
            if let Node::Lvasgn(Lvasgn {
                value: Some(value),
                operator_l: Some(operator_l),
                ..
            }) = statement
            {
                if let Node::Send(Send {
                    method_name,
                    selector_l: Some(selector_l),
                    recv: Some(recv),
                    ..
                }) = *value
                {
                    if method_name == "-@" && operator_l.end == selector_l.begin {
                        if let Node::Lvar(Lvar { expression_l, .. }) = *recv {
                            if selector_l.end < expression_l.begin {
                                line_col(&input, selector_l, "warning: ambiguous assignment");
                            }
                        }
                    }
                }
            }
        }
    }
}

fn line_col(input: &DecodedInput, loc: Loc, msg: &str) {
    let (begin_line, _) = input.line_col_for_pos(loc.begin).unwrap();
    let line_no = begin_line;
    let line = &input.lines[line_no];
    let line_loc = Loc {
        begin: line.start,
        end: line.line_end(),
    };
    let line = line_loc.source(&input).unwrap();
    let (_, start_col) = input.line_col_for_pos(loc.begin).unwrap();

    let filename = &input.name;
    let prefix = format!("{}:{}:{}", filename, line_no + 1, start_col + 1);

    let highlight = format!(
        "{indent}{tildes}",
        indent = " ".repeat(start_col),
        tildes = if loc.size() > 0 {
            "^".repeat(loc.size())
        } else {
            "".to_string()
        }
    );
    println!("{}\n  --> {}\n\n{}\n{}", msg, prefix, line, highlight);
}
