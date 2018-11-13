#![feature(exclusive_range_pattern)]

#[macro_use]
extern crate lazy_static;
extern crate rand;

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use hlt::log::Log;
use hlt::navi::Navi;
use rand::Rng;
use rand::SeedableRng;
use rand::XorShiftRng;
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::collections::HashMap;

mod hlt;

#[derive(PartialEq, Debug)]
enum ShipStates {
Exploring,
Returning,
}



fn main() {
    let args: Vec<String> = env::args().collect();
    let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    };
    let seed_bytes: Vec<u8> = (0..16).map(|x| ((rng_seed >> (x % 8)) & 0xFF) as u8).collect();
    let mut rng: XorShiftRng = SeedableRng::from_seed([
        seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
        seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
        seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
        seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15]
    ]);

    let mut ship_status = HashMap::new();
    let mut game = Game::new();
    let mut navi = Navi::new(game.map.width, game.map.height);
    // At this point "game" variable is populated with initial map data.
    // This is a good place to do computationally expensive start-up pre-processing.
    // As soon as you call "ready" function below, the 2 second per turn timer will start.
    Game::ready("MyRustBot");

    Log::log(&format!("Successfully created bot! My Player ID is {}. Bot rng seed is {}.", game.my_id.0, rng_seed));

    loop {
        game.update_frame();
        navi.update_frame(&game);

        let me = &game.players[game.my_id.0];
        let map = &mut game.map;

        let mut command_queue: Vec<Command> = Vec::new();

        let divider = match game.turn_number {
            1..20 => 4,
            21..40 => 5,
            41..60 => 6,
            61..100 => 8,
            101..200 => 10,
            _ => 20
        };

        for ship_id in &me.ship_ids {
            let ship = &game.ships[ship_id];
            let cell = map.at_entity(ship);

            
            if !ship_status.contains_key(ship_id){
                ship_status.insert(ship_id.clone(), ShipStates::Exploring); 
            }

            if ship_status[ship_id] == ShipStates::Returning {
                if &cell.position == &me.shipyard.position {
                    ship_status.remove(&ship_id);
                    ship_status.entry(*ship_id).or_insert(ShipStates::Exploring);
                }
                else {
                    let direction = navi.naive_navigate(&ship, &me.shipyard.position);
                    command_queue.push(ship.move_ship(direction));
                    continue;
                }
            }
            else if ship.halite >= ((game.constants.max_halite as f32) * 0.98) as usize{
                Log::log(&format!("Ship {} have more than 980 halite, returning", ship_id.0));
                ship_status.remove(&ship_id);
                ship_status.entry(*ship_id).or_insert(ShipStates::Returning);
            }

            let command = if cell.halite < game.constants.max_halite / divider {
                let random_direction = Direction::get_all_cardinals();
                let mut max_halite_dir = Direction::Still;
                let mut max_halite = 0;
                for possible_direction in random_direction {
                    let target_pos = &ship.position.directional_offset(possible_direction);
                    if navi.is_safe(&target_pos){
                        if map.at_position(target_pos).halite > max_halite {
                            max_halite = map.at_position(target_pos).halite;
                            max_halite_dir = possible_direction;
                        }
                    }
                }
                if max_halite <= ((cell.halite as f32) * 1.03) as usize{
                    navi.mark_unsafe(&ship.position, ship.id);
                    ship.stay_still()
                }
                else{
                let target_pos = ship.position.directional_offset(max_halite_dir);
                let direction = navi.naive_navigate(&ship, &target_pos); //max_halite_dir)
                ship.move_ship(direction)
                }
            } else {
                navi.mark_unsafe(&ship.position, ship.id);
                ship.stay_still()

            };
            command_queue.push(command);
        }

        /*
        if (480 - game.turn_number) <= 30 { //destroy ships
            Log::log("Random death!");
            let mut command_queue: Vec<Command> = Vec::new(); //Overide previous rules
            for ship_id in &me.ship_ids {
                let ship = &game.ships[ship_id];
                let cell = map.at_entity(ship);
                navi.mark_safe(&me.shipyard.position);
                let direction = navi.naive_navigate(&ship, &me.shipyard.position);
                let target_pos = ship.position.directional_offset(direction);
                if target_pos.x == me.shipyard.position.x && target_pos.y == me.shipyard.position.y{
                    command_queue.push(ship.move_ship(direction));
                    break;
                }
            }
            Log::log(&format!("Steps: {:?}", command_queue));

        }*/

        if
            game.turn_number <= 150 &&
            me.halite >= game.constants.ship_cost &&
            navi.is_safe(&me.shipyard.position)
        {
            command_queue.push(me.shipyard.spawn());
        }


        Game::end_turn(&command_queue);
    }
}
