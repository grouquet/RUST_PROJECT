use std::io::{self, Write};
use std::time::{Duration, Instant};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType},
};

#[derive(Clone, Copy, Eq, PartialEq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Thing {
    Wall,
    Pellet,
    Empty
}
struct Game {
    pacman_pos: Position, // position du Pacman
    width: i32, // largeur de la grille
    length: i32, // longueur de la grille
    pacman_dir: Position, // direction du Pacman
    map: Vec<Vec<Thing>>, // la carte du jeu
    score : u32, // le score du joueur
}

impl Game {
    //on crée une nouvelle instance de jeu à parir d'une map lisible
    fn from_ascii(ascii: &[&str]) -> Self {
        let length = ascii.len() as i32;
        let width = ascii[0].chars().count() as i32;

        let mut map = vec![vec![Thing::Empty; width as usize]; length as usize]; //initialisier grille vide
        let mut pacman_pos = Position { x: 0, y: 0 }; // initialiser position pacman

        for (y, line) in ascii.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                map[y][x] = match ch {
                    '#' => Thing::Wall,
                    '.' => Thing::Pellet,
                    'P' => {
                        pacman_pos = Position { x: x as i32, y: y as i32 };
                        Thing::Empty
                    },
                    _ => Thing::Empty,
                };
            }
        }

        Self {
            pacman_pos,
            width,
            length,
            pacman_dir: Position { x: 0, y: 0 },
            map,
            score: 0,
        }
    }

    fn in_bounds(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.length
    }

    //donne le type à une position donnée
    fn thing(&self, pos: Position) -> Thing {
        self.map[pos.y as usize][pos.x as usize]
    }

    //met un type à une position donnée
    fn set_thing(&mut self, pos: Position, thing: Thing) {
        self.map[pos.y as usize][pos.x as usize] = thing;
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

        let next = Position { x: shift_x, y: shift_y };

        if !self.in_bounds(next) {
            return; // ne bouge pas si hors limites ou mur
        }

        if self.thing(next) == Thing::Wall {
            return; // ne bouge pas si mur
        }

        self.pacman_pos = next; //déplacement du pacman

        if self.thing(next) == Thing::Pellet {
            self.score += 1; // incrémente le score
            self.set_thing(next, Thing::Empty); // enlève le pellet
        }
    }

    fn render (&self, out: &mut impl Write) -> io::Result<()>  {

        queue!(out, cursor::MoveTo(0, 0))?;  

        queue!(out, Print(format!("Score: {}\r\n", self.score)))?;

        //Dessin --> boucles imbriquées 
        for y in 0..self.length {
            for x in 0..self.width {
                let pos = Position { x, y };
                let pacman = if pos == self.pacman_pos {
                    "C" // dessine Pacman
                } else {
                    match self.thing(pos) {
                        Thing::Wall => "#",    // dessine un mur
                        Thing::Pellet => ".",  // dessine un pellet
                        Thing::Empty => " "    // dessine un espace vide
                    }
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

    // Map ASCII : # murs, . pastilles, P spawn
    const MAP: &[&str] = &[
        "####################",
        "#P.................#",
        "#.####.######.####.#",
        "#......#....#......#",
        "#.####.#.##.#.####.#",
        "#......#....#......#",
        "#.####.######.####.#",
        "#..................#",
        "####################",
    ];

    //preparation du terminal
    terminal::enable_raw_mode()?; // pas de line buffering
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    //jeu
    let mut game = Game::from_ascii(MAP); // jeu à partir de MAP

    //tick rate
    let frame_duration = Duration::from_millis(50);
    let mut last = Instant::now();

    //boucle principale
    'game_loop: loop {
        //temps restant avant la prochaine frame
        let elapsed = last.elapsed();
        let timeout = frame_duration.saturating_sub(elapsed);

        //gestion des entrées
        if event::poll(timeout)? {
            if let Event::Key(event) = event::read()? {
                if event.code == KeyCode::Esc || event.code == KeyCode::Char('x') {
                    break 'game_loop; // quitter le jeu
                }
                game.handle_input(event.code);
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
