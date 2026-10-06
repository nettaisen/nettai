--!strict
-- What an EXE6 link battle picks at random with its settings (`sub_81209DC`, twice a round for the two rounds after:
-- `PosRNG1()` for the stage, then `PosRNG2()` for the background). Whoever makes a random match picks from these
-- (the engine picks none: a round's settings state its stage and background); a match may name any link battle stage
-- and any background all the same.

local stages = require("../stages")

return {
    -- The stages a link battle picks one of at random with its settings, by the pick's index (`sub_81209DC`:
    -- `PosRNG1() % 0x60` into `BattleSettingsList1`, its first 96 records, each a stage of its own). Whoever makes a
    -- random match picks from them; a match may still name any link battle stage.
    stages = {
        stages["netbattle-1"], stages["netbattle-2"], stages["netbattle-3"], stages["netbattle-4"], stages["netbattle-5"], stages["netbattle-6"],
        stages["netbattle-7"], stages["netbattle-8"], stages["netbattle-9"], stages["netbattle-10"], stages["netbattle-11"], stages["netbattle-12"],
        stages["netbattle-13"], stages["netbattle-14"], stages["netbattle-15"], stages["netbattle-16"], stages["netbattle-17"], stages["netbattle-18"],
        stages["netbattle-19"], stages["netbattle-20"], stages["netbattle-21"], stages["netbattle-22"], stages["netbattle-23"], stages["netbattle-24"],
        stages["netbattle-25"], stages["netbattle-26"], stages["netbattle-27"], stages["netbattle-28"], stages["netbattle-29"], stages["netbattle-30"],
        stages["netbattle-31"], stages["netbattle-32"], stages["netbattle-33"], stages["netbattle-34"], stages["netbattle-35"], stages["netbattle-36"],
        stages["netbattle-37"], stages["netbattle-38"], stages["netbattle-39"], stages["netbattle-40"], stages["netbattle-41"], stages["netbattle-42"],
        stages["netbattle-43"], stages["netbattle-44"], stages["netbattle-45"], stages["netbattle-46"], stages["netbattle-47"], stages["netbattle-48"],
        stages["netbattle-49"], stages["netbattle-50"], stages["netbattle-51"], stages["netbattle-52"], stages["netbattle-53"], stages["netbattle-54"],
        stages["netbattle-55"], stages["netbattle-56"], stages["netbattle-57"], stages["netbattle-58"], stages["netbattle-59"], stages["netbattle-60"],
        stages["netbattle-61"], stages["netbattle-62"], stages["netbattle-63"], stages["netbattle-64"], stages["netbattle-65"], stages["netbattle-66"],
        stages["netbattle-67"], stages["netbattle-68"], stages["netbattle-69"], stages["netbattle-70"], stages["netbattle-71"], stages["netbattle-72"],
        stages["netbattle-73"], stages["netbattle-74"], stages["netbattle-75"], stages["netbattle-76"], stages["netbattle-77"], stages["netbattle-78"],
        stages["netbattle-79"], stages["netbattle-80"], stages["netbattle-81"], stages["netbattle-82"], stages["netbattle-83"], stages["netbattle-84"],
        stages["netbattle-85"], stages["netbattle-86"], stages["netbattle-87"], stages["netbattle-88"], stages["netbattle-89"], stages["netbattle-90"],
        stages["netbattle-91"], stages["netbattle-92"], stages["netbattle-93"], stages["netbattle-94"], stages["netbattle-95"], stages["netbattle-96"],
    },
    -- How many of them, from the first, a set's first round picks among: the count the comm menu passes for the first
    -- round goes by the match type (`sub_812A76C`: a single battle's mode 0, a triple battle's mode 1, a random
    -- battle's mode 2, `dword_8120A14`'s 0x44, 0x60 and 0x44 past the records' 0x60), and a random match is a triple
    -- battle's practice (nettai-match's effects, `off_812B7AC`'s 0x600): 0x60, all of them, as the rounds after.
    first_round_stages = 96,
    -- The backgrounds a link battle picks one of at random with its settings (`sub_81209DC`: `byte_8120A20`, 21
    -- entries, in the table's order; three are there twice, so twice as likely). Whoever makes a random match picks
    -- from them (the engine picks none: a round's settings state its background).
    backgrounds = {
        asset.background("lans-hp"), asset.background("acdc-hp"), asset.background("acdc-hp"),
        asset.background("aquarium-hp"), asset.background("sky-hp"), asset.background("green-hp"),
        asset.background("robot-control-comp"), asset.background("comp"), asset.background("comp-alt"),
        asset.background("central-area"), asset.background("aquarium-comp"), asset.background("seaside-area"),
        asset.background("judgetree-comp"), asset.background("green-area"), asset.background("sky-area"),
        asset.background("undernet"), asset.background("mr-weather-comp"), asset.background("underground"),
        asset.background("underground"), asset.background("acdc-area"), asset.background("acdc-area"),
    },
}
