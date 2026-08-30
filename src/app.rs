use std::any;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::layout::Spacing;
use ratatui::prelude::*;
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Wrap};
use crate::data;

#[derive(Default)]
pub struct App {
    app_data: data::Data,
    screen: Screen,
    exit: bool,
    saved_main_state: Option<MainScreenState>
}

pub enum Screen {
    Main(MainScreenState),
    Note(NoteScreenState)
}
#[derive(Default)]
pub struct MainScreenState {
    search: String,
    list_state: ListState
}

#[derive(Default)]
pub enum NotePanes {
    #[default]
    Note,
    Todo,
}

#[derive(Default)]
pub struct NoteScreenState {
    edit: bool,
    focused: NotePanes,
    selected_todo: usize,
    name: String,
    body: String,
    todos: Vec<data::Todo>,
    list_state: ListState
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Main(MainScreenState::default())
    }
}

impl MainScreenState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            search: String::new(),
            list_state,
        }
    }

    pub fn filtered_notes<'a>(&self, notes: &'a [data::Note]) -> Vec<&'a data::Note> {
        if self.search.is_empty() {
            notes.iter().collect()
        } else {
            notes
                .iter()
                .filter(|n| n.name.to_lowercase().contains(&self.search.to_lowercase()))
                .collect()
        }
    }

    pub fn selected_note_name(&self, notes: &[data::Note]) -> Option<String> {
        let index = self.list_state.selected()?;
        let filtered = self.filtered_notes(notes);

        filtered.get(index).map(|note| note.name.clone())
    }

    pub fn select_up(&mut self) {
        self.list_state.select_previous();
    }

    pub fn select_down(&mut self) {
        self.list_state.select_next();
    }
}

impl App {
    pub fn new(app_data: data::Data) -> Self {
        Self {
            app_data,
            exit: false,
            screen: Default::default(),
            saved_main_state: None
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        self.screen = Self::main_screen();
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        match &mut self.screen {
            Screen::Main(main_state) => Self::render_main_screen(area, buf, main_state, &self.app_data),
            Screen::Note(note_state) => Self::render_note_screen(area, buf, note_state, &self.app_data),
        }
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('x') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.exit(),
            _ => {}
        }

        match &mut self.screen {
            Screen::Main(state) => {
                if let Some(next_screen) = Self::handle_main_keys(key_event, state, &self.app_data) {
                    if let Screen::Main(old_state) = std::mem::take(&mut self.screen) {
                        self.saved_main_state = Some(old_state);
                    }
                    self.screen = next_screen;
                }
            },
            Screen::Note(state) => {
                if state.edit {
                    Self::handle_note_edit_keys(key_event)
                } else {
                    Self::handle_note_view_keys(key_event)
                }
            }
        }
    }

    fn handle_main_keys(key_event: KeyEvent, state: &mut MainScreenState, app_data: &data::Data) -> Option<Screen> {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => state.select_down(),
            KeyCode::Char('k') | KeyCode::Up => state.select_up(),
            KeyCode::Enter => {
                if let Some(note_name) = state.selected_note_name(&app_data.notes) {
                    state.search.clear();
                    return Some(Self::note_screen(note_name, app_data, false))
                }
            }
            _ => {}
        }
        None
    }

    fn handle_note_edit_keys(key_event: KeyEvent) {

    }

    fn handle_note_view_keys(key_event: KeyEvent) {

    }

    fn exit(&mut self) {
        match self.screen {
            Screen::Note(ref mut note_screen) => {
                if note_screen.edit {
                    note_screen.edit = false;
                } else {
                    if let Some(previous_state) = self.saved_main_state.take() {
                        self.screen = Screen::Main(previous_state);
                    } else {
                        self.screen = Self::main_screen();
                    }
                }
            },
            Screen::Main(_) => self.exit = true
        }
    }

    fn main_screen() -> Screen {
        Screen::Main(MainScreenState::new())
    }

    fn note_screen(note: String, app_data: &data::Data, edit: bool) -> Screen {
        let note_data = app_data.notes.iter()
            .find(|n| n.name == note)
            .cloned()
            .unwrap_or_default();

        let body = note_data.body;
        let todos = note_data.todos;

        Screen::Note(NoteScreenState {
            edit,
            name: note,
            body,
            todos,
            ..NoteScreenState::default()
        })
    }

    fn render_main_screen(area: Rect, buf: &mut Buffer, state: &mut MainScreenState, app_data: &data::Data) {
        let title = Line::from(" Scribe ".bold());
        let instructions = Line::from(vec![
            " ^S Search |".into(),
            " ↑↓ Navigate |".into(),
            " ↵ Open |".into(),
            " ^X Quit ".into(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .borders(Borders::ALL);

        let items: Vec<ListItem> = app_data.notes
            .iter()
            .map(|note| ListItem::new(note.name.as_str()))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Modifier::REVERSED);

        let [_, center, _] = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ]).areas(area);

        StatefulWidget::render(list, center, buf, &mut state.list_state);
    }

    fn render_note_screen(area: Rect, buf: &mut Buffer, state: &mut NoteScreenState, app_data: &data::Data) {
        // inner wasn't rendering the title how i liked, so split like this instead
        let [left, right] = Layout::horizontal([Constraint::Fill(1); 2])
            .spacing(Spacing::Overlap(1))
            .areas(area);

        let title = Line::from(format!(" {} ", state.name));

        let note_block = Block::bordered()
            .title(title.centered())
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::horizontal(1));

        let todo_block = Block::bordered()
            .title(Line::from(" Todo ").centered())
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::right(1));

        let body = Paragraph::new(state.body.clone())
            .wrap(Wrap { trim: false })
            .block(note_block);

        let items: Vec<ListItem> = state.todos.iter()
            .filter(|t| t.pinned)
            .chain(state.todos.iter().filter(|t| !t.pinned))
            .map(|todo| {
                let temp = if todo.done {
                    format!("[✓] {}", todo.text)
                } else {
                    format!("[ ] {}", todo.text)
                };
                if todo.pinned {
                    format!("*{}", temp)
                } else {
                    format!(" {}", temp)
                }
            })
            .map(|text| ListItem::new(text))
            .collect();

        let list = List::new(items)
            .block(todo_block)
            .highlight_style(Modifier::REVERSED);

        body.render(left, buf);
        StatefulWidget::render(list, right, buf, &mut state.list_state);
    }
}