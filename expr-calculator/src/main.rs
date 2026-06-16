use std::{collections::HashMap, env, process::exit};

use reedline::{
    ColumnarMenu, DefaultCompleter, DefaultPrompt, Emacs, ExampleHighlighter, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    default_emacs_keybindings,
};
use uipc_expr::Expr;

fn main() {
    let args: Vec<String> = env::args().collect();
    let no_vars = &HashMap::new();
    if args.len() > 1 {
        let in_expr = args[1..].join(" ");
        match Expr::parse(&in_expr) {
            Ok(s) => {
                let v = s.eval(no_vars);
                println!("{}", v);
                exit(0);
            }
            Err(_) => {
                println!("Unable to parse expression");
                exit(1);
            }
        }
    }

    let commands = vec![
        "+".into(),
        "-".into(),
        "*".into(),
        "/".into(),
        "\\".into(),
        "%".into(),
        "imod".into(),
        "^".into(),
        "&".into(),
        "|".into(),
        "==".into(),
        "!=".into(),
        "<".into(),
        ">".into(),
        "<=".into(),
        ">=".into(),
        "abs".into(),
        "round".into(),
        "floor".into(),
        "ceil".into(),
        "min".into(),
        "max".into(),
        "neg".into(),
        "sqrt".into(),
        "not".into(),
        "sin".into(),
        "cos".into(),
        "atan2".into(),
        "dup".into(),
        "swap".into(),
        "?".into(),
        "PI".into(),
        "E".into(),
    ];
    let completer = Box::new(DefaultCompleter::new(commands.clone()));
    // Use the interactive menu to select options from the completer
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    // Set up the required keybindings
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    println!();
    println!("Available commands: {}", commands.clone().join(", "));
    println!("Comparisons return 1 for true and 0 for false");
    println!("Ternary `?`: cond then else ? — nonzero cond picks then");
    println!("Stack: `dup` duplicates top, `swap` swaps top two");
    println!("Trig functions (sin, cos, atan2) operate in radians");
    println!();
    let mut line_editor = Reedline::create()
        .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    let mut prompt = DefaultPrompt::default();
    prompt.left_prompt = reedline::DefaultPromptSegment::Empty;
    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => match Expr::parse(buffer.as_str()) {
                Ok(s) => {
                    let v = s.eval(no_vars);
                    println!("= {}", v);
                }
                Err(_) => println!("Unable to parse expression"),
            },
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("\nAborted!");
                break;
            }
            x => {
                println!("Event: {:?}", x);
            }
        }
    }
}
