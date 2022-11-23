use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
    
extern crate rustpat;

use std::{
    error::Error, io, time::{Duration, Instant}, thread, path::Path, fs::read_dir,
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Corner, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap, Tabs, List, ListItem, ListState},
    Frame, Terminal,
};

struct StatefulList<T> {
    state: ListState,
    songs: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(songs: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            songs,
        }
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.songs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.songs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

struct App {
    songs: StatefulList<String>,
    dir: String,
    paused: bool,
    currentsong: String,
}

impl App {
    fn new() -> App {
        App {
            songs: StatefulList::with_items(vec![("Proccessing".to_owned())]),
            dir: "Null".to_owned(),
            paused: false,
            currentsong: "Nothing Playing".to_owned(),
        }
    }

    fn play(&mut self) {
        let singer = self.songs.state.selected().expect("Failed").to_string();
        let i: u32 = singer.parse().unwrap();
         let arg = format!("{}/Music/{}", home::home_dir().expect("Home is null").display(), self.songs.songs.remove(i as usize));
         let extras = format!("{}/Music/", home::home_dir().expect("Home is null").display());
          let song = &arg.replace(&extras, "");
           thread::spawn(move || {
               let pat = rustpat::PAT::new().unwrap();
                pat.play(&arg).unwrap();
           });
        self.currentsong = song.to_string();
        self.paused = false;
         thread::sleep(std::time::Duration::from_millis(300));
        self.update();
    }

    fn pause(&mut self) {
        let pat = rustpat::PAT::new().unwrap();
         pat.pause().unwrap();
        self.paused = true;
    }

    fn unpause(&mut self) {
        let pat = rustpat::PAT::new().unwrap();
         pat.resume().unwrap();
        self.paused = false;
    }

    fn setup(&mut self) {
        self.songs.next();
    }

    fn update(&mut self) {
       let dir = format!("{}/Music/", home::home_dir().expect("Home is null").display());
        let paths = read_dir(&Path::new(dir.as_str())).unwrap();
        
        let songs =
        paths.filter_map(|entry| {
          entry.ok()
            .and_then(|e| e.path().file_name()
            .and_then(|n| n.to_str().map(String::from))
          )
        }).collect::<Vec<String>>();
        self.songs = StatefulList::with_items(songs);
        self.dir = dir.to_string();
        self.setup();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
   
    let mut tick_rate = Duration::from_millis(250);
    let mut songrate = Duration::from_millis(500);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate, songrate);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut tick_rate: Duration,
    mut songrate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut song_tick = Instant::now();
    let pat = rustpat::PAT::new().unwrap();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.songs.next(),
                    KeyCode::Up => app.songs.previous(),
                    KeyCode::Enter => app.play(),
                    KeyCode::Char('p') => app.pause(),
                    KeyCode::Char('o') => app.unpause(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.setup();
            tick_rate = Duration::from_secs(3600);
            last_tick = Instant::now();
        }
        if song_tick.elapsed() >= songrate {
            app.update();
            songrate = Duration::from_secs(3600);
            song_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(size);

        let songs: Vec<ListItem> = app
        .songs
        .songs
        .iter()
        .map(|i| {
            let log = Spans::from(vec![Span::raw(i)]);

            ListItem::new(vec![
                Spans::from("-".repeat(chunks[1].width as usize)),
                log,
            ])
        })
        .collect();
    
        let player = List::new(songs)
            .block(Block::default().borders(Borders::ALL).title("Songs"))
            .start_corner(Corner::BottomLeft)
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            );
    
        f.render_stateful_widget(player, chunks[0], &mut app.songs.state);

        let currentsong = format!("Song: {}", app.currentsong);
        let paused = format!("Paused: {}", app.paused);
let text = vec![
    Spans::from(currentsong),
    Spans::from(paused),
];

let create_block = |title| {
    Block::default()
        .borders(Borders::ALL)
        .style(Style::default())
        .title(Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        ))
};

let paragraph = Paragraph::new(text.clone())
.style(Style::default())
.block(create_block("Player Info"))
.alignment(Alignment::Left)
.wrap(Wrap { trim: true });
f.render_widget(paragraph, chunks[1]);
}
