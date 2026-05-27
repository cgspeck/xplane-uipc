use std::collections::HashMap;

use reedline::{
    ColumnarMenu, DefaultCompleter, DefaultPrompt, Emacs, ExampleHighlighter, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    default_emacs_keybindings,
};
use uipc_expr::Expr;

fn main() {
    let commands = vec![
        "+".into(),
        "-".into(),
        "*".into(),
        "/".into(),
        "\\".into(),
        "%".into(),
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
        "?".into(),
        "PI".into(),
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
    println!("Avaliable commands: {}", commands.clone().join(", "));
    println!("Logical comparators return 1 for true and 0 for false");
    println!("Or `|` and ternery `?` consider any value > 0 to be true ");
    println!("");
    let mut line_editor = Reedline::create()
        .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    let prompt = DefaultPrompt::default();
    let no_vars = &HashMap::new();
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
