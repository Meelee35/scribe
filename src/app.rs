use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui::layout::Spacing;
use ratatui::prelude::*;
use ratatui::symbols::merge::MergeStrategy;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding};
use ratatui_textarea::{CursorMove, TextArea, WrapMode};
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
    name: String,
    body: TextArea<'static>,
    todos: Vec<data::Todo>,
    list_state: ListState,
    todo_input: Option<TodoInput>
}

enum TodoInput {
    New(TextArea<'static>),
    Edit(TodoEdit),
}

struct TodoEdit {
    text_area: TextArea<'static>,
    index: usize,
}

impl Default for TodoInput {
    fn default() -> Self {
        TodoInput::New(TextArea::default())
    }
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Main(MainScreenState::default())
    }
}

enum NoteViewAction {
    ToggleDone(usize),
    TogglePin(usize),
}

impl NoteScreenState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            list_state,
            ..Default::default()
        }
    }

    pub fn select_up(&mut self) {
        self.list_state.select_previous();
    }

    pub fn select_down(&mut self) {
        self.list_state.select_next();
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
                self.handle_key_event(key_event)?
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match key_event.code {
            KeyCode::Char('x') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.exit()?,
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
                    Self::handle_note_edit_keys(key_event, state, &mut self.app_data);
                } else {
                    let sorted: Vec<usize> = (0..state.todos.len())
                        .filter(|&i| state.todos[i].pinned)
                        .chain((0..state.todos.len()).filter(|&i| !state.todos[i].pinned))
                        .collect();

                    if let Some(action) = Self::handle_note_view_keys(key_event, state, &mut self.app_data) {
                        if let Some(&real_index) = sorted.get(match action {
                            NoteViewAction::ToggleDone(i) | NoteViewAction::TogglePin(i) => i,
                        }) {
                            match action {
                                NoteViewAction::ToggleDone(_) => state.todos[real_index].done = !state.todos[real_index].done,
                                NoteViewAction::TogglePin(_) => state.todos[real_index].pinned = !state.todos[real_index].pinned,
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }


    fn handle_main_keys(key_event: KeyEvent, state: &mut MainScreenState, app_data: &data::Data) -> Option<Screen> {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => state.select_down(),
            KeyCode::Char('k') | KeyCode::Up => state.select_up(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(note_name) = state.selected_note_name(&app_data.notes) {
                    state.search.clear();
                    return Some(Self::note_screen(note_name, app_data, false))
                }
            }
            _ => {}
        }
        None
    }

    fn handle_note_edit_keys(key_event: KeyEvent, state: &mut NoteScreenState, app_data: &mut data::Data) -> Option<Screen> {
        match key_event.code {
            KeyCode::Char('e') if key_event.modifiers.contains(KeyModifiers::CONTROL) => { Self::toggle_edit_mode(state, app_data).expect("rip"); None },
            KeyCode::Char('z') if key_event.modifiers.contains(KeyModifiers::CONTROL) => { Self::discard_edit_mode(state, app_data); None },
            KeyCode::Tab => { Self::toggle_edit_focus(state); None },
            _ => {
                if let NotePanes::Note = state.focused {
                    state.body.input(key_event);
                } else {
                    match key_event.code {
                        KeyCode::Char('j') | KeyCode::Down => state.select_down(),
                        KeyCode::Char('k') | KeyCode::Up => state.select_up(),
                        KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            let sorted = sorted_todo_indices(&state.todos);
                            if let Some(display_index) = state.list_state.selected() {
                                if let Some(&real_index) = sorted.get(display_index) {
                                    state.todos[real_index].pinned = !state.todos[real_index].pinned;
                                }
                            }
                        },
                        KeyCode::Enter => {
                            if let Some(input) = state.todo_input.take() {
                                match input {
                                    TodoInput::Edit(edit_state) => {
                                        let text = edit_state.text_area.lines()[0].clone();
                                        if text.is_empty() {
                                            state.todos.remove(edit_state.index);
                                        } else {
                                            state.todos[edit_state.index].text = text;
                                        }
                                    }
                                    TodoInput::New(textarea) => {
                                        let text = textarea.lines()[0].clone();
                                        if !text.is_empty() {
                                            state.todos.push(data::Todo {
                                                text,
                                                ..data::Todo::default()
                                            });
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::Char('n') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            let mut textarea = TextArea::default();
                            textarea.move_cursor(CursorMove::End);
                            textarea.set_cursor_line_style(Style::default());
                            state.todo_input = Some(TodoInput::New(textarea));
                        }
                        _ => {
                            let sorted = sorted_todo_indices(&state.todos);

                            if let Some(TodoInput::Edit(edit_state)) = &mut state.todo_input {
                                edit_state.text_area.input(key_event);
                            } else if let Some(TodoInput::New(textarea)) = &mut state.todo_input {
                                textarea.input(key_event);
                            } else if let Some(display_index) = state.list_state.selected() {
                                if let Some(&real_index) = sorted.get(display_index) {
                                    let mut textarea = TextArea::from([state.todos[real_index].text.as_str()]);
                                    textarea.move_cursor(CursorMove::End);
                                    textarea.set_cursor_line_style(Style::default());
                                    textarea.input(key_event);
                                    state.todo_input = Some(TodoInput::Edit(TodoEdit { text_area: textarea, index: real_index }));
                                }
                            }
                        }
                    }
                }
                None
            },
        }
    }

    fn handle_note_view_keys(key_event: KeyEvent, state: &mut NoteScreenState, data: &mut data::Data) -> Option<NoteViewAction> {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => { state.select_down(); None }
            KeyCode::Char('k') | KeyCode::Up => { state.select_up(); None }
            KeyCode::Char(' ') | KeyCode::Enter => state.list_state.selected().map(NoteViewAction::ToggleDone),
            KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => state.list_state.selected().map(NoteViewAction::TogglePin),
            KeyCode::Char('e') if key_event.modifiers.contains(KeyModifiers::CONTROL) => { Self::toggle_edit_mode(state, data).expect("rip"); None }

            _ => None
        }
    }

    fn discard_edit_mode(state: &mut NoteScreenState, app_data: &data::Data) {
        state.todo_input = None;
        state.edit = false;

        if let Some(note) = app_data.notes.iter().find(|n| n.name == state.name) {
            state.body = TextArea::from(note.body.lines().collect::<Vec<&str>>());
            state.todos = note.todos.clone();
        }

        state.list_state.select(Some(0));
        state.body.set_cursor_style(Style::default());
        state.body.set_cursor_line_style(Style::default());
    }

    fn toggle_edit_mode(state: &mut NoteScreenState, app_data: &mut data::Data) -> anyhow::Result<()> {
        if !state.edit {
            state.edit = true;
        } else {
            state.edit = false;
            app_data.notes = app_data.notes.iter().map(|n| {
                if n.name == state.name {
                    data::Note { body: state.body.lines().join("\n"), todos: state.todos.clone(), ..n.clone() }
                } else {
                    n.clone()
                }
            }).collect();
            data::save(app_data)?;
        }

        if state.edit {
            match state.focused {
                NotePanes::Note => {
                    state.body.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                    state.list_state.select(None);
                }
                NotePanes::Todo => {
                    state.list_state.select(Some(0));
                    state.body.set_cursor_style(Style::default());
                }
            }
        } else {
            state.list_state.select(Some(0));
            state.body.set_cursor_style(Style::default());
        }

        Ok(())
    }

    fn toggle_edit_focus(state: &mut NoteScreenState) {
        state.focused = match state.focused {
            NotePanes::Note => {
                state.list_state.select(Some(0));
                state.body.set_cursor_style(Style::default());
                NotePanes::Todo
            }
            NotePanes::Todo => {
                state.body.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                state.list_state.select(None);
                NotePanes::Note
            }
        };
    }

    fn exit(&mut self) -> anyhow::Result<()> {
        match self.screen {
            Screen::Note(ref mut note_screen) => {
                self.app_data.notes = self.app_data.notes.iter().map(|n| {
                    if n.name == note_screen.name {
                        data::Note { body: note_screen.body.lines().join("\n"), todos: note_screen.todos.clone(), ..n.clone() }
                    } else {
                        n.clone()
                    }
                }).collect();
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
        data::save(&mut self.app_data)?;
        Ok(())
    }

    fn main_screen() -> Screen {
        Screen::Main(MainScreenState::new())
    }

    fn note_screen(note: String, app_data: &data::Data, edit: bool) -> Screen {
        let note_data = app_data.notes.iter()
            .find(|n| n.name == note)
            .cloned()
            .unwrap_or_default();

        let mut body = TextArea::from(note_data.body.lines().collect::<Vec<&str>>());
        let todos = note_data.todos;

        body.set_wrap_mode(WrapMode::Word);
        body.set_cursor_style(Style::default());
        body.set_cursor_line_style(Style::default());

        Screen::Note(NoteScreenState {
            edit,
            name: note,
            body,
            todos,
            ..NoteScreenState::new()
        })
    }

    fn render_main_screen(area: Rect, buf: &mut Buffer, state: &mut MainScreenState, app_data: &data::Data) {
        let title = Line::from(" todo ".bold());
        let instructions = Line::from(" ^S Search | ↑↓ Navigate | ↵ Open | ^X Quit ");

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1));

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

        let [mut todo_list_area, todo_input_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
        ]).spacing(Spacing::Overlap(1)).areas(right);

        let mut note_pane_title = Line::from(format!(" {} ", state.name));
        let mut todo_pane_title = Line::from(" Todo ");

        let mut instructions = " ↑↓ Navigate | ↵ Toggle | ^P Pin | ^E Edit | ^X Back ";

        if state.edit {
            instructions = match state.focused {
                NotePanes::Note => {
                    note_pane_title = Line::from(" [EDITING] ");
                    " ↹ Switch | ^E Done | ^Z Discard |^X Back "
                },
                NotePanes::Todo => {
                    todo_pane_title = Line::from(" [EDITING] ");
                    " ↹ Switch | ↑↓ Navigate | ⋯ Edit | ^P Pin | ^E Done | ^Z Discard | ^X Back "
                },
            }
        }

        let edit_block = Block::default()
            .title(Line::from(" Type todo text ").centered())
            .borders(Borders::ALL);

        if state.edit {
            let text_area = match &mut state.todo_input {
                Some(TodoInput::Edit(input_state)) => Some(&mut input_state.text_area),
                Some(TodoInput::New(textarea)) => Some(textarea),
                None => None,
            };
            if let Some(textarea) = text_area {
                textarea.set_block(edit_block);
                textarea.render(todo_input_area, buf);
            } else {
                todo_list_area = right;
            }
        } else {
            todo_list_area = right;
        }

        let instruction_block = Block::default()
            .title_bottom(Line::from(instructions).centered())
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Replace);

        let note_block = Block::bordered()
            .title(note_pane_title.centered())
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::horizontal(1));

        let todo_block = Block::bordered()
            .title(todo_pane_title.centered())
            .borders(Borders::ALL)
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::right(1));

        state.body.set_block(note_block);

        let items: Vec<ListItem> = state.todos.iter()
            .filter(|t| t.pinned)
            .chain(state.todos.iter().filter(|t| !t.pinned))
            .map(|todo| {
                if !state.edit {
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
                } else {
                    if todo.pinned {
                        format!(" * {}", todo.text)
                    } else {
                        format!(" - {}", todo.text)
                    }
                }
            })
            .map(|text| ListItem::new(text))
            .collect();

        let list = List::new(items)
            .block(todo_block)
            .highlight_style(Modifier::REVERSED);

        instruction_block.render(area, buf);
        state.body.render(left, buf);
        StatefulWidget::render(list, todo_list_area, buf, &mut state.list_state);
    }
}

fn sorted_todo_indices(todos: &[data::Todo]) -> Vec<usize> {
    (0..todos.len())
        .filter(|&i| todos[i].pinned)
        .chain((0..todos.len()).filter(|&i| !todos[i].pinned))
        .collect()
}