use std::io::{self, Write}; 
use std::time::{Duration, Instant};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue,
    style::Print,
    terminal::{self},
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

#[derive(Clone, Copy)]
struct Ghost {
    pos: Position,
    dir: Position,
}

struct Game {
    pacman_pos: Position, // position du Pacman
    width: i32, // largeur de la grille
    length: i32, // longueur de la grille
    pacman_dir: Position, // direction du Pacman
    map: Vec<Vec<Thing>>, // la carte du jeu
    score : u32, // le score du joueur
    pellets_left: u32, // pastilles restantes -> pour gérer la victoire
    wanted_dir: Position, // direction voulue par le joueur -> pour conserver la direction si on peut pas tourner
    game_over: bool, // état de fin de jeu
    tick_count: u64, // compteur de ticks pour deplacer fantome (1tick sur 2)
    ghosts : Vec<Ghost>, // liste des fantômes
}

impl Game {
    //on crée une nouvelle instance de jeu à parir d'une map lisible
    fn from_ascii(ascii: &[&str]) -> Self {
        let length = ascii.len() as i32;
        let width = ascii[0].chars().count() as i32;

        let mut map = vec![vec![Thing::Empty; width as usize]; length as usize]; //initialisier grille vide
        let mut pacman_pos = Position { x: 0, y: 0 }; // initialiser position pacman
        let wanted_dir = Position { x: 0, y: 0 }; // initialiser direction voulue
        let mut ghosts: Vec<Ghost> = Vec::new(); // initialiser liste des fantômes

        let mut pellets_left: u32 = 0;

        for (y, line) in ascii.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                map[y][x] = match ch {
                    '#' => Thing::Wall,
                    '.' => {
                        pellets_left += 1;
                        Thing::Pellet
                    },
                    'P' => {
                        pacman_pos = Position { x: x as i32, y: y as i32 };
                        Thing::Empty
                    },
                    'G' => {
                        ghosts.push(Ghost {
                            pos: Position { x: x as i32, y: y as i32 },
                            dir: Position { x: 0, y: 0 },
                        });
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
            pellets_left,
            wanted_dir,
            game_over: false,
            tick_count: 0,
            ghosts,
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

    // Gère les entrées clavier pour changer la direction du Pacman, la wanted_dir
    fn handle_input(&mut self, code: KeyCode) {
        self.wanted_dir = match code {
            KeyCode::Up | KeyCode::Char('z') => Position { x: 0, y: -1 }, // haut
            KeyCode::Down | KeyCode::Char('s') => Position { x: 0, y: 1 }, // bas
            KeyCode::Left | KeyCode::Char('q') => Position { x: -1, y: 0 }, // gauche
            KeyCode::Right | KeyCode::Char('d') => Position { x: 1, y: 0 }, // droite
            _ => self.wanted_dir, // on ne reset pas !
        };
    }

    fn next_position(&self, from: Position, dir: Position) -> Position {
        Position {
            x: from.x + dir.x,
            y: from.y + dir.y,
        }
    }

    fn can_move(&self, from: Position, dir: Position) -> bool {
        if dir.x == 0 && dir.y == 0 {
            return false; // pas de mouvement
        }
        let next = self.next_position(from, dir);
        self.in_bounds(next) && self.thing(next) != Thing::Wall
    }
    
    fn manhattan_distance(a: Position, b: Position) -> i32 {
        (a.x - b.x).abs() + (a.y - b.y).abs()
    }

    fn opposite(dir: Position) -> Position {
        Position { x: -dir.x, y: -dir.y }
    }

    fn move_ghost(&mut self) {
        for g in &mut self.ghosts {
            Game::move_one_ghost(g, self.pacman_pos, self.width, self.length, &self.map);
        }
    }


    fn move_one_ghost(ghost: &mut Ghost, pacman_pos: Position, width: i32, length: i32, map: &Vec<Vec<Thing>>) {
        // Logique de déplacement du fantôme
        let dirs = [
            Position { x: 0, y: -1 },
            Position { x: 0, y: 1 },
            Position { x: -1, y: 0 },
            Position { x: 1, y: 0 },
        ];

        let mut candidates: Vec<Position> = Vec::new(); // directions possibles
        for dir in dirs.iter() {
            let next = Position {
                x: ghost.pos.x + dir.x,
                y: ghost.pos.y + dir.y,
            };
            if next.x >= 0 && next.x < width && next.y >= 0 && next.y < length && map[next.y as usize][next.x as usize] != Thing::Wall {
                candidates.push(*dir);
            }
        }

        if candidates.is_empty() {
            return; // pas de mouvement possible
        }

        //ce bloc filtre les directions pour éviter les demi-tours répétitifs
        let opp = Self::opposite(ghost.dir);
        let mut filtered: Vec<Position> = if ghost.dir.x == 0 && ghost.dir.y == 0 {
            candidates.clone() // pas de direction précédente, on garde toutes les options
        } else {
            candidates
                .iter()
                .copied()
                .filter(|&d| d != opp) //on enlève la direction opposée
                .collect()
        };

        if filtered.is_empty() {
            filtered = candidates; // autoriser le demi-tour si nécessaire
        }


        let mut best_dir = filtered[0];
        let mut best_distance = i32::MAX;

        //on choisit la direction qui rapproche le plus du pacman
        for dir in filtered {

            let next = Position {
                x: ghost.pos.x + dir.x,
                y: ghost.pos.y + dir.y,
            };
            let dist = Self::manhattan_distance(next, pacman_pos);

            if dist < best_distance {
                best_distance = dist;
                best_dir = dir;
            }

        }
        ghost.pos = Position {
            x: ghost.pos.x + best_dir.x,
            y: ghost.pos.y + best_dir.y,
        };
        ghost.dir = best_dir;//on garde en mémoire la direction pour après (éviter les demi-tours cf l.176)
    }

    // Met à jour la position du Pacman en fonction de sa direction
    fn update(&mut self) {

        if self.game_over || self.pellets_left == 0 {
            return; // ne fait rien si le jeu est terminé
        }

        let prev_pos = self.pacman_pos; //permet de gérer si pac et un ghost se croise au même tick

        //si la direction voulue est possible, on la prend
        if self.can_move(self.pacman_pos, self.wanted_dir) {
            self.pacman_dir = self.wanted_dir; // met à jour la direction voulue
        }

        let mut moved = false;
        let mut next = self.pacman_pos;
        //on choisit la direction actuelle qui a pu être mi sà jour juste au dessus
        if self.can_move(self.pacman_pos, self.pacman_dir) {
            next = self.next_position(self.pacman_pos, self.pacman_dir);
            self.pacman_pos = next; //déplacement du pacman
            moved = true;
        }
        
        //si fantome rencontre pacman
        if self.ghosts.iter().any(|g| g.pos == self.pacman_pos) {
            self.game_over = true; // fin du jeu
            return;
        }

        self.tick_count = self.tick_count.wrapping_add(1);
        
        if self.tick_count % 2 == 0 {
            let prev_ghost: Vec<Position> = self.ghosts.iter().map(|g| g.pos).collect();
            self.move_ghost(); // déplace le fantôme tous les 2 ticks comme ça ils sont plus lents
            if self.ghosts.iter().enumerate().any(|(i, g)| {
                g.pos == self.pacman_pos || (prev_ghost[i] == self.pacman_pos && g.pos == prev_pos)}) 
                { 
                self.game_over = true; // fin du jeu
                return;
            }
        };

        if moved && self.thing(next) == Thing::Pellet {
            self.score += 1; // incrémente le score
            self.pellets_left = self.pellets_left.saturating_sub(1); // décrémente les pastilles restantes
            self.set_thing(next, Thing::Empty); // enlève le pellet
        }
    }

    fn render (&self, out: &mut impl Write) -> io::Result<()>  {

        queue!(out, cursor::MoveTo(0, 0))?;  

        queue!(out, Print(format!("Score: {}\r\n  |  Pellets left: {}\r\n", self.score, self.pellets_left)))?;

        if self.game_over {
            queue!(out, Print("Game Over! Press 'Esc' or 'x' to exit.\r\n"))?;
        } else if self.pellets_left == 0 {
            queue!(out, Print("You win! Press 'Esc' or 'x' to exit.\r\n"))?;
        } else {
            queue!(out, Print("\r\n"))?; // ligne vide entre le score et la grille
        }

        //Dessin --> boucles imbriquées 
        for y in 0..self.length {
            for x in 0..self.width {
                let pos = Position { x, y };
                let pacman = if pos == self.pacman_pos {
                    "C" // dessine Pacman
                } else if self.ghosts.iter().any(|g| g.pos == pos) {
                    "G" // dessine le fantôme
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

    const MAP: &[&str] = &[
        "########################################",
        "#P..............##.....................#",
        "#.####.#####.###.##.###.#####.####.###.#",
        "#......#...#................#...#......#",
        "######.#.#.########.##.########.#.######",
        "#......#.#....#.....##.....#....#......#",
        "#.##########.#.###.####.###.#.########.#",
        "#............#........G.....#..........#",
        "#.##########.#####.##.#####.##########.#",
        "#......#...........##...........#......#",
        "######.#.#########.##.#########.#.######",
        "#......#.....#........#.....#...#......#",
        "#.##########.#.######.######.#.#########",
        "#..........#.#..............#.#........#",
        "#.########.#.#######.######.#.########.#",
        "#......G...#.....##.....##..#..........#",
        "########################################",
    ];



    //preparation du terminal
    terminal::enable_raw_mode()?; // pas de line buffering
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?; //other screen buffer

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
                if event.code == KeyCode::Char('r') {
                    game = Game::from_ascii(MAP); // redémarrer le jeu
                    execute!(stdout, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
                    game.render(&mut stdout)?; // enlève l'ancien GameOver
                    last = Instant::now();
                    continue 'game_loop;
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
