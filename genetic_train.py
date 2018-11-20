import random 
import os
import sys
import subprocess
import json
from trueskill import Rating, quality_1vs1, rate_1vs1, rate
from tqdm import tqdm
import collections

sizes = [32, 40, 48, 56, 64]

map_size = sizes[2]
players_num = 4

if len(sys.argv) > 2:
    map_size = sizes[int(sys.argv[1])]
    players_num = int(sys.argv[2])


bot_values_dict = collections.OrderedDict()


bot_values_dict["filter_divider"] = 2
bot_values_dict["dropoff_turn"] = 150
bot_values_dict["reserve_dropoff_halite"] = 18
bot_values_dict["dropoff_distance_penalty"] =78
bot_values_dict["dropoff_group_send"] = 12
bot_values_dict["search_radius"] = 1
bot_values_dict["random_death_turn"] = 20
bot_values_dict["exploring_move_multiplier"] = 130
bot_values_dict["stop_producing_ships_turn"] = 170





def gen_player(parameters):
    base_string = "\"RUST_BACKTRACE=1 ./target/release/my_bot"
    for key, value in parameters.items():
        base_string += " " + str(value)
    base_string += "\""
    
    return [base_string, Rating(), parameters]



def gen_match(size, bots_list):
    string = "./halite --no-timeout --no-replay --no-logs -v --results-as-json --width {0} --height {0} ".format(size)
    for bot in bots_list:
        string += bot[0] + " "
    return string

generation = 0

while True:
    print("Generation: ", generation, "Mapsize:", map_size, "Player num:", players_num)
    #generate variants
    variants = [bot_values_dict.copy()]
    for i in range(0,10):
        temp_dict = bot_values_dict.copy()
        for key, value in temp_dict.items():
            if random.random() > 0.5:
                temp_dict[key] = int(value * (1 + random.normalvariate(0,10)/100))
        variants.append(temp_dict.copy())


    global_bots_list = []
    for variant in variants:
        global_bots_list.append(gen_player(variant))


    for i in tqdm(range(0,1000)):
        bots_num = random.sample(range(0,len(global_bots_list)), players_num)
        if players_num == 2:
            bots = [global_bots_list[bots_num[0]], global_bots_list[bots_num[1]]]#, global_bots_list[bots_num[2]], global_bots_list[bots_num[3]]]
        else:
            bots = [global_bots_list[bots_num[0]], global_bots_list[bots_num[1]], global_bots_list[bots_num[2]], global_bots_list[bots_num[3]]]
        command = gen_match(map_size, bots)
        b = subprocess.Popen(command, stdout=subprocess.PIPE, shell=True)
        out, err = b.communicate()
        if players_num == 2:
            bot_0_rank = json.loads(out.decode())['stats']["0"]["rank"]
            if bot_0_rank == 1: #so he won
                new_r1, new_r2 = rate_1vs1(bots[0][1], bots[1][1])
            else:
                new_r2, new_r1 = rate_1vs1(bots[1][1], bots[0][1])

            global_bots_list[bots_num[0]][1] = new_r1
            global_bots_list[bots_num[1]][1] = new_r2

            for bot, key in json.loads(out.decode())['terminated'].items():
                if key == True:
                    global_bots_list.pop(bots_num[int(bot)])
                    break

        else:
            # print(out.decode())
            bot_0_rank = json.loads(out.decode())['stats']["0"]["rank"]
            bot_1_rank = json.loads(out.decode())['stats']["1"]["rank"]
            bot_2_rank = json.loads(out.decode())['stats']["2"]["rank"]
            bot_3_rank = json.loads(out.decode())['stats']["3"]["rank"]

            rating_group = []
            for i in range(1,5):
                if bot_0_rank == i:
                    rating_group.append((global_bots_list[bots_num[0]][1],))
                if bot_1_rank == i:
                    rating_group.append((global_bots_list[bots_num[1]][1],))
                if bot_2_rank == i:
                    rating_group.append((global_bots_list[bots_num[2]][1],))
                if bot_3_rank == i:
                    rating_group.append((global_bots_list[bots_num[3]][1],))

            #for bot in global_bots_list:
            #    print([bot[2], (bot[1].mu - 3*bot[1].sigma), bot[1]])

            new_rate = rate(rating_group)
            for i in range(1,5):
                if bot_0_rank == i:
                    global_bots_list[bots_num[0]][1] = new_rate[i-1][0]
                if bot_1_rank == i:
                    global_bots_list[bots_num[1]][1] = new_rate[i-1][0]
                if bot_2_rank == i:
                    global_bots_list[bots_num[2]][1] = new_rate[i-1][0]
                if bot_3_rank == i:
                    global_bots_list[bots_num[3]][1] = new_rate[i-1][0]


            #for bot in global_bots_list:
            #    print([bot[2], (bot[1].mu - 3*bot[1].sigma), bot[1]])

            # check if some of bots were terminated and drop them
            for bot, key in json.loads(out.decode())['terminated'].items():
                if key == True:
                    global_bots_list.pop(bots_num[int(bot)])
                    break


    converted_list = []
    for bot in global_bots_list:
        item = [bot[2], (bot[1].mu - 3*bot[1].sigma), bot[1]]
        #print(item)
        converted_list.append(item)

    #print("Sorted list")
    sorted_list = sorted(converted_list, key=lambda x: x[1])
    print("Generation", generation, "winner:", sorted_list[-1])
    bot_values_dict = sorted_list[-1][0].copy()
    generation += 1
    #print(b)

    #if quality_1vs1(r1, r2) > 0.50:
    #new_r1, new_r2 = rate_1vs1(r1, r2)
    #print(b)
