use iced::widget::container::*;
use iced::widget::{container, row, column, text, button, text_editor, text_input, checkbox, scrollable};
use iced::Theme;

use static_analysis::domains::interval::INF;
use static_analysis::semantics::State;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .theme(Theme::Dracula)
        .run()
}


#[derive(Default)]
struct App {
    params: Parameters,
    output: Output
}

#[derive(Clone)]
enum Message {
    ToggleWidening(bool),
    NewWidening(u32),
    NewNarrowing(u32),
    NewM(i64),
    NewN(i64),
    Edit(text_editor::Action),
    SetProg(String, String),
    ExecAnalysis
}


impl App {
    fn view(&self) -> scrollable::Scrollable<Message> {
        scrollable(column![
            self.params.view(),
            self.output.view()
        ].spacing(20).padding(10)
        ).direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new()))
            .spacing(1)
        
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleWidening(b) => self.params.perform_widening = b,
            Message::NewWidening(n) => self.params.widening = n,
            Message::NewNarrowing(n) => self.params.narrowing = n,
            Message::NewM(n) => self.params.domain = (n, self.params.domain.1),
            Message::NewN(n) => self.params.domain = (self.params.domain.0, n),
            Message::Edit(a) => self.params.content.perform(a),
            Message::SetProg(filename, prog) => {
                self.params.filename = filename;
                self.params.content.perform(text_editor::Action::SelectAll);
                self.params.content.perform(text_editor::Action::Edit(text_editor::Edit::Backspace)); 
                self.params.content.perform(text_editor::Action::Edit(text_editor::Edit::Paste(prog.into()))); 
            },
            Message::ExecAnalysis => {
                let full_prog = self.params.content.text();
                let idx = full_prog.find("===").unwrap_or(0);
                let (prog, init_state) = if idx == 0 {
                    (&full_prog[..], None)
                } else {
                    (
                        &full_prog[idx+3..],
                        Some(State::from_str(&full_prog[..idx]).unwrap_or(State::new()))
                    )
                };

                static_analysis::domains::interval::set_bounds(self.params.domain.0, self.params.domain.1);
                self.output.output = static_analysis::cfg::execute::<static_analysis::domains::Interval>(
                    prog,
                    init_state,
                    if self.params.perform_widening { Some(self.params.widening) } else { None },
                    if self.params.perform_widening { Some(self.params.narrowing) } else { None }
                );
            }
        }
    }
}

struct Parameters {
    filename: String,
    content: text_editor::Content,
    perform_widening: bool,
    widening: u32,
    narrowing: u32,
    domain: (i64, i64),
}

impl Default for Parameters {
    fn default() -> Parameters {
        Parameters {
            filename: String::default(),
            content: text_editor::Content::default(),
            perform_widening: bool::default(),
            widening: u32::default(),
            narrowing: u32::default(),
            domain: (-INF, INF)
        }
    }
}

impl Parameters {
    fn view(&self) -> Container<Message> {
        let filename_field = text_input("No File", &self.filename);
        let open_file_button = button("Open File")
            .on_press_with(|| {
                let filepath = rfd::FileDialog::new()
                    .add_filter("While files", &["while"])
                    .set_directory(std::env::current_dir().expect("No working dir??"))
                    .pick_file();
                let prog = filepath.clone()
                    .map(|p| std::fs::read_to_string(p).unwrap_or("File read error".to_string()))
                    .unwrap_or("File selection error".to_string());

                Message::SetProg(
                    filepath.unwrap_or_default()
                        .to_str()
                        .expect("Non unicode path????")
                        .to_string(),
                    prog
                )
            });

        let editor = text_editor(&self.content)
            .font(iced::Font::MONOSPACE)
            .wrapping(text::Wrapping::Word)
            .placeholder("Enter your code here...")
            .padding(5)
            .on_action(Message::Edit);

        let toggle_field = checkbox(self.perform_widening)
            .label("Perform widening")
            .on_toggle(Message::ToggleWidening);

        let widen_field = text_input("0", &self.widening.to_string())
            .width(50)
            .on_input_maybe(self.perform_widening.then_some(
                    |x: String| Message::NewWidening(x.parse().unwrap_or(0))
            ));
        let narrow_field = text_input("0", &self.narrowing.to_string())
            .width(50)
            .on_input_maybe(self.perform_widening.then_some(
                    |x: String| Message::NewNarrowing(x.parse().unwrap_or(0))
            ));

        let (m_str, n_str) = (
            if self.domain.0 == -INF { "-∞" } else { &self.domain.0.to_string() },
            if self.domain.1 == INF { "+∞" } else { &self.domain.1.to_string() }
        );
        let m_field = text_input("", m_str)
            .width(100)
            .on_input(|x| Message::NewM(x.parse().unwrap_or(-INF)));
        let n_field = text_input("", n_str)
            .width(100)
            .on_input(|x| Message::NewN(x.parse().unwrap_or(INF)));

        container(column![
            column![
                row![filename_field , open_file_button],
                editor,
            ],
            row![text("Domain: "), text("[").size(20), m_field, text(",").size(20), n_field, text("]").size(20)].spacing(5),
            container(column![
                toggle_field,
                row![text("Widening delay: "), widen_field],
                row![text("Narrowing steps: "), narrow_field],
            ].spacing(5).padding(5)
            ).style(container::bordered_box),
            button("Analyse").on_press(Message::ExecAnalysis),
        ].spacing(10))
    }
}


#[derive(Default)]
struct Output {
    output: String
}


impl Output {
    fn view(&self) -> Container<Message> {
        container(column![
            text(&self.output).font(iced::Font::MONOSPACE)
        ]).style(container::bordered_box)
            .width(iced::Fill)
            .padding(5)
    }
}
