use std::io::{self, Write};
use std::time::{Duration, Instant};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType},
};

#[derive(Clone, Copy)]
struct Position {
    x: i32,
    y: i32,
}

struct Game {
    pacman_pos: Position, // position du Pacman
    width: i32, // largeur de la grille
    length: i32, // longueur de la grille
    pacman_dir: Position, // direction du Pacman
}

impl Game {
    fn new(width: i32, length: i32) -> Self {
        Self {
            pacman_pos: Position { x: width / 2, y: length / 2 }, // position init du Pacman au centre de la grille
            width,
            length,
            pacman_dir: Position { x: 0, y: 0 }, // direction init du Pacman (immobile)
        }
    }

    // Gère les entrées clavier pour changer la direction du Pacman
    fn handle_input(&mut self, code: KeyCode) {
        self.pacman_dir = match code {
            KeyCode::Up | KeyCode::Char('z') => Position { x: 0, y: -1 }, // haut
            KeyCode::Down | KeyCode::Char('s') => Position { x: 0, y: 1 }, // bas
            KeyCode::Left | KeyCode::Char('q') => Position { x: -1, y: 0 }, // gauche
            KeyCode::Right | KeyCode::Char('d') => Position { x: 1, y: 0 }, // droite
            _ => self.pacman_dir,
        };
    }
    // Met à jour la position du Pacman en fonction de sa direction
    fn update(&mut self) {
        let shift_x = self.pacman_pos.x + self.pacman_dir.x;
        let shift_y = self.pacman_pos.y + self.pacman_dir.y;

        if shift_x >= 0 && shift_x < self.width {
            self.pacman_pos.x = shift_x;
        }
        if shift_y >= 0 && shift_y < self.length {
            self.pacman_pos.y = shift_y;
        }
    }

    fn render (&self, out: &mut impl Write) -> io::Result<()>  {
        // on efface et redessine tout, c'est ok pour petite grille
        queue!(out, cursor::MoveTo(0, 0), Clear(ClearType::All))?;  

        //Dessin --> boucles imbriquées 
        for y in 0..self.length {
            for x in 0..self.width {
                let pacman = if x == self.pacman_pos.x && y == self.pacman_pos.y {
                    "C" // dessine Pacman
                } else {
                    "." // dessine un point vide
                };
                queue!(out, Print(pacman))?;
            }
            queue!(out, Print("\r\n"))?;
        }
        out.flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {

    //preparation du terminal
    terminal::enable_raw_mode()?; // pas de line buffering
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    //jeu
    let mut game = Game::new(20, 10); // grille 20x10

    //tick rate
    let frame_duration = Duration::from_millis(16);
    let mut last = Instant::now();

    //boucle principale
    'game_loop: loop {
        //gestion des entrées
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(event) => {
                    if event.code == KeyCode::Esc || event.code == KeyCode::Char('x') {
                        break 'game_loop; // quitter le jeu
                    }
                    game.handle_input(event.code);
                }
                _ => {}
            }

        }

        //update à chaque tick
        if last.elapsed() >= frame_duration {
            game.update();
            game.render(&mut stdout)?;
            last = Instant::now();
        }
    }

    //restauration du terminal
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
