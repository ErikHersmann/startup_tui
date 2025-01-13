use derive_setters::Setters;
use chrono;
use color_eyre::Result;
use text_io::read;
use crossterm::{event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers}, terminal::{self}};
use ratatui::{
    buffer::Buffer, layout::{Constraint, Direction, Layout, Rect}, style::{
        palette::tailwind::{self, SLATE},
        Color, Modifier, Style, Stylize,
    }, text::{Line, Span, Text}, widgets::{Block, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Table, Widget, Wrap}, DefaultTerminal, Frame
};
// use serde_json::Value;
use chrono_tz::Tz;
use std::{collections::HashMap, fs, io::BufReader, process::exit};

const TODO_HEADER_STYLE: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;






#[derive(Debug, Default, Setters)]
struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    border_style: Style,
    title_style: Style,
    style: Style,
}

impl Widget for Popup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ensure that all cells under the popup are cleared to avoid leaking content
        Clear.render(area, buf);
        let block = Block::new()
            .title(self.title)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style);
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}


#[derive(Debug, Default)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    colors: TableColors,
    application_state: ApplicationState,
}
#[derive(Debug, Default, PartialEq)]
enum ApplicationState {
    #[default]
    Main,
    InsertRunPopup,
    InsertCalendarItemPopup,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_crossterm_events()?;
            if self.application_state != ApplicationState::Main {
                match terminal.draw(|frame| self.popup(frame)) {
                    Ok(res) => {
                        self.application_state = ApplicationState::Main;
                    },
                    Err(e) => {return Err(e.into())}
                };
            }
        }
        Ok(())
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            (KeyModifiers::CONTROL, KeyCode::Char('l') | KeyCode::Char('L')) => {
                self.modify_todo_list_popup()
            }
            _ => {}
        }
    }

    fn modify_todo_list_popup(&mut self) {
        self.application_state = ApplicationState::InsertCalendarItemPopup;
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn popup(&mut self, frame: &mut Frame) {
        let popup = Popup::default()
        .content("Hello world!")
        .style(Style::new().yellow())
        .title("With Clear")
        .title_style(Style::new().white().bold())
        .border_style(Style::new().red());
        frame.render_widget(popup, Rect { x: 10, y: 10, width: 5, height: 5 });

    }

    fn ui(&mut self, f: &mut Frame) {
        ////////////
        // LAYOUT //
        ////////////
        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(vec![Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(f.area());
        let layout_left_side = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(0)
            .spacing(0)
            .constraints(vec![Constraint::Fill(1), Constraint::Percentage(15)])
            .split(layout_main[0]);
        let layout_left_bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(65), Constraint::Fill(1)])
            .split(layout_left_side[1]);
        let layout_left_right_bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout_left_bottom[1]);


        //////////////
        // DATETIME //
        //////////////
        let utc_now = chrono::Utc::now();
        let ohio_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::US::Eastern);
        let berlin_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::Europe::Berlin);
        let tokyo_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::Asia::Tokyo);
        let datetime_text: Vec<Line<'_>> = vec![
            vec![Span::styled(
                ohio_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    + " "
                    + &ohio_time.timezone().to_string(),
                Style::default().fg(Color::Yellow),
            )]
            .into(),
            vec![Span::styled(
                berlin_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    + " "
                    + &berlin_time.timezone().to_string(),
                Style::default().fg(Color::Yellow),
            )]
            .into(),
            vec![Span::styled(
                tokyo_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    + " "
                    + &tokyo_time.timezone().to_string(),
                Style::default().fg(Color::Yellow),
            )]
            .into(),
        ];

        ////////////////
        // TODO LIST  //
        ////////////////
        let todo_item_style = Style::default().fg(Color::Yellow);
        // .bg(Color::LightCyan);
        let mut todo_list_text: Vec<Line<'_>> =
            vec![Span::styled("TODO", TODO_HEADER_STYLE).into()];
        let environment_dict = App::get_environment_dict();
        if let Some(todo_items) = environment_dict["todo_list"].as_array() {
            for (index, todo_item) in todo_items.iter().enumerate() {
                if let Some(todo_item_inner) = todo_item.as_str() {
                    todo_list_text.push(
                        Span::styled("\t".to_string() + todo_item_inner, todo_item_style).into(),
                    );
                    // println!("Todo item {}: {}", index, todo_item);
                } else {
                    println!("todo list item not a string");
                }
            }
        } else {
            println!("todo list items do not exist.");
        }

        ////////////////
        // TABLE STUFF//
        ////////////////
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = [
            "",
            "月曜日",
            "火曜日",
            "水曜日",
            "木曜日",
            "金曜日",
            "土曜日",
            "日曜日",
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);
        let bar = " █ ";

        let rows = [
            Row::new(vec!["Dawn", "7:12-7:42"]),
            Row::new(vec!["Dusk", "20:12-20:50"]),
            Row::new(vec!["Precip", "20%/30%"]),
            Row::new(vec!["Humidity", "80%"]),
            Row::new(vec!["Temperature", "7°C"]),
            Row::new(vec!["Training AM", "90 Easy"]),
            Row::new(vec!["Training PM", "Rest"]),
        ];
        let widths = [
            Constraint::Length(14),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ];
        let table_bottom_left = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        ////////////
        // WIDGETS//
        ////////////
        f.render_widget(
            Paragraph::new(todo_list_text).block(Block::default().borders(Borders::ALL)),
            layout_main[1],
        );
        f.render_widget(
            table_bottom_left
                .block(Block::default().borders(Borders::RIGHT | Borders::LEFT | Borders::BOTTOM)),
            layout_left_bottom[0],
        );
        f.render_widget(
            Paragraph::new("").block(Block::default().borders(Borders::ALL)),
            layout_left_side[0],
        );
        f.render_widget(
            Paragraph::new("今週 今月 今年")
                .block(Block::default().borders(Borders::BOTTOM | Borders::RIGHT | Borders::LEFT)),
            layout_left_right_bottom[0],
        );
        f.render_widget(
            Paragraph::new(datetime_text)
                .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)),
            layout_left_right_bottom[1],
        );
    }

    fn get_environment_dict() -> serde_json::Value {
        // let mut return_value: [[String; 7]; 2] = Default::default();
        // Read the AM for the next 7 days from the file
        let file = match fs::File::open(hello_user::ENVIRONMENT_PATH_JSON) {
            Ok(res) => res,
            Err(e) => {
                println!("{}", e.to_string());
                return Default::default();
            }
        };
        let reader = BufReader::new(file);
        let trainings_dict: serde_json::Value = match serde_json::from_reader(reader) {
            Ok(res) => res,
            Err(e) => {
                println!("{}", e.to_string());
                return Default::default();
            }
        };
        return trainings_dict;
        // let poop = trainings_dict["running_schedule"]["1/12/2005"].clone();
        // return_value[0][0] = "test".to_string();

        // return return_value;
    }
}

enum TimeOfDay {
    AM,
    PM,
}
