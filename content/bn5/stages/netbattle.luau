--!strict
-- BN5's netbattle stages: the battle settings records its link battles take a stage from (0x0811AF4C, 95 records,
-- BN6's `BattleSettingsList1` netbattle records' counterpart), each distinct one a stage, with its panel layout
-- (0x0800BD6C) and its actor list (the navis). A record's numbers (its places in the list, its layout's number
-- and its actor list's address) are compat's. Waiting for the engine (bn5-map.md §15.3 item 1) or the port:
--   netbattle-3 (layout 0x00, actors 0x0811b6fd): obstacles (actor types 8)
--   netbattle-4 (layout 0x00, actors 0x0811b70e): obstacles (actor types 8)
--   netbattle-15 (layout 0x00, actors 0x0811b71f): obstacles (actor types 8)
--   netbattle-16 (layout 0x00, actors 0x0811b730): obstacles (actor types 8)
--   netbattle-17 (layout 0x00, actors 0x0811b741): obstacles (actor types 8)
--   netbattle-18 (layout 0x00, actors 0x0811b752): obstacles (actor types 8)
--   netbattle-21 (layout 0x20, actors 0x0811b653): obstacles (actor types 3)
--   netbattle-22 (layout 0x20, actors 0x0811b664): obstacles (actor types 3)
--   netbattle-25 (layout 0x25, actors 0x0811b675): obstacles (actor types 3)
--   netbattle-26 (layout 0x25, actors 0x0811b686): obstacles (actor types 3)
--   netbattle-27 (layout 0x25, actors 0x0811b697): obstacles (actor types 3)
--   netbattle-28 (layout 0x25, actors 0x0811b6a8): obstacles (actor types 3)
--   netbattle-29 (layout 0x26, actors 0x0811b6b9): obstacles (actor types 3)
--   netbattle-30 (layout 0x26, actors 0x0811b6ca): obstacles (actor types 3)
--   netbattle-31 (layout 0x26, actors 0x0811b6db): obstacles (actor types 3)
--   netbattle-32 (layout 0x26, actors 0x0811b6ec): obstacles (actor types 3)
--   netbattle-33 (layout 0x77, actors 0x0811b61d): metal
--   netbattle-34 (layout 0x77, actors 0x0811b626): metal
--   netbattle-35 (layout 0x7e, actors 0x0811b641): metal
--   netbattle-36 (layout 0x7e, actors 0x0811b64a): metal
--   netbattle-37 (layout 0x7d, actors 0x0811b62f): metal
--   netbattle-38 (layout 0x7d, actors 0x0811b638): metal
--   netbattle-39 (layout 0x40, actors 0x0811b641): sea
--   netbattle-40 (layout 0x40, actors 0x0811b64a): sea
--   netbattle-41 (layout 0x42, actors 0x0811b61d): sea
--   netbattle-42 (layout 0x42, actors 0x0811b626): sea
--   netbattle-43 (layout 0x44, actors 0x0811b61d): sea
--   netbattle-44 (layout 0x44, actors 0x0811b626): sea
--   netbattle-57 (layout 0x70, actors 0x0811b641): lava
--   netbattle-58 (layout 0x70, actors 0x0811b64a): lava
--   netbattle-59 (layout 0x72, actors 0x0811b61d): lava
--   netbattle-60 (layout 0x72, actors 0x0811b626): lava
--   netbattle-69 (layout 0x46, actors 0x0811b61d): sea
--   netbattle-70 (layout 0x46, actors 0x0811b626): sea
--   netbattle-75 (layout 0xad, actors 0x0811b641): sea
--   netbattle-76 (layout 0xad, actors 0x0811b64a): sea
--   netbattle-77 (layout 0x78, actors 0x0811b61d): metal
--   netbattle-78 (layout 0x78, actors 0x0811b626): metal
--   netbattle-79 (layout 0x9f, actors 0x0811b641): metal
--   netbattle-80 (layout 0x9f, actors 0x0811b64a): metal
--   netbattle-81 (layout 0x54, actors 0x0811b763): obstacles (actor types 8)
--   netbattle-82 (layout 0x54, actors 0x0811b774): obstacles (actor types 8)
--   netbattle-83 (layout 0x00, actors 0x0811b785): obstacles (actor types 9)
--   netbattle-84 (layout 0x00, actors 0x0811b796): obstacles (actor types 9)
--   netbattle-89 (layout 0x10, actors 0x0811b7a7): obstacles (actor types 8)
--   netbattle-90 (layout 0x10, actors 0x0811b7b8): obstacles (actor types 8)
--   netbattle-91 (layout 0x20, actors 0x0811b7c9): obstacles (actor types 9)
--   netbattle-92 (layout 0x20, actors 0x0811b7da): obstacles (actor types 9)
--   netbattle-93 (layout 0x6a, actors 0x0811b7eb): obstacles (actor types 3)
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

return {
    netbattle_1 = define.stage {
        id = "netbattle-1",
        layout = {
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_2 = define.stage {
        id = "netbattle-2",
        layout = {
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_5 = define.stage {
        id = "netbattle-5",
        layout = {
            "normal cracked cracked cracked cracked normal",
            "normal normal normal normal normal normal",
            "cracked cracked normal normal cracked cracked",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_6 = define.stage {
        id = "netbattle-6",
        layout = {
            "normal cracked cracked cracked cracked normal",
            "normal normal normal normal normal normal",
            "cracked cracked normal normal cracked cracked",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_7 = define.stage {
        id = "netbattle-7",
        layout = {
            "normal normal normal normal normal normal",
            "cracked cracked cracked cracked cracked cracked",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 1 }, { place = "navi", side = 1, x = 5, y = 1 } },
    },
    netbattle_8 = define.stage {
        id = "netbattle-8",
        layout = {
            "normal normal normal normal normal normal",
            "cracked cracked cracked cracked cracked cracked",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 1 }, { place = "navi", side = 0, x = 2, y = 1 } },
    },
    netbattle_9 = define.stage {
        id = "netbattle-9",
        layout = {
            "cracked cracked normal normal cracked cracked",
            "normal normal normal normal normal normal",
            "cracked cracked normal normal cracked cracked",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_10 = define.stage {
        id = "netbattle-10",
        layout = {
            "cracked cracked normal normal cracked cracked",
            "normal normal normal normal normal normal",
            "cracked cracked normal normal cracked cracked",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_11 = define.stage {
        id = "netbattle-11",
        layout = {
            "missing normal normal normal normal missing",
            "normal normal normal normal normal normal",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_12 = define.stage {
        id = "netbattle-12",
        layout = {
            "missing normal normal normal normal missing",
            "normal normal normal normal normal normal",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_13 = define.stage {
        id = "netbattle-13",
        layout = {
            "normal normal normal normal normal normal",
            "normal missing normal normal missing normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 1, y = 2 }, { place = "navi", side = 1, x = 6, y = 2 } },
    },
    netbattle_14 = define.stage {
        id = "netbattle-14",
        layout = {
            "normal normal normal normal normal normal",
            "normal missing normal normal missing normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 6, y = 2 }, { place = "navi", side = 0, x = 1, y = 2 } },
    },
    netbattle_19 = define.stage {
        id = "netbattle-19",
        layout = {
            "normal normal missing missing normal normal",
            "normal normal normal normal normal normal",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_20 = define.stage {
        id = "netbattle-20",
        layout = {
            "normal normal missing missing normal normal",
            "normal normal normal normal normal normal",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_23 = define.stage {
        id = "netbattle-23",
        layout = {
            "normal normal grass grass normal normal",
            "normal grass grass grass grass normal",
            "normal normal grass grass normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_24 = define.stage {
        id = "netbattle-24",
        layout = {
            "normal normal grass grass normal normal",
            "normal grass grass grass grass normal",
            "normal normal grass grass normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_45 = define.stage {
        id = "netbattle-45",
        layout = {
            "normal ice ice ice ice normal",
            "normal ice ice ice ice normal",
            "normal ice ice ice ice normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_46 = define.stage {
        id = "netbattle-46",
        layout = {
            "normal ice ice ice ice normal",
            "normal ice ice ice ice normal",
            "normal ice ice ice ice normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_47 = define.stage {
        id = "netbattle-47",
        layout = {
            "missing ice ice ice ice ice",
            "ice ice ice ice ice ice",
            "ice ice ice ice ice missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_48 = define.stage {
        id = "netbattle-48",
        layout = {
            "missing ice ice ice ice ice",
            "ice ice ice ice ice ice",
            "ice ice ice ice ice missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_49 = define.stage {
        id = "netbattle-49",
        layout = {
            "ice ice ice ice ice missing",
            "ice ice ice ice ice ice",
            "missing ice ice ice ice ice",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_50 = define.stage {
        id = "netbattle-50",
        layout = {
            "ice ice ice ice ice missing",
            "ice ice ice ice ice ice",
            "missing ice ice ice ice ice",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_51 = define.stage {
        id = "netbattle-51",
        layout = {
            "ice ice normal normal ice ice",
            "ice ice normal normal ice ice",
            "ice ice normal normal ice ice",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_52 = define.stage {
        id = "netbattle-52",
        layout = {
            "ice ice normal normal ice ice",
            "ice ice normal normal ice ice",
            "ice ice normal normal ice ice",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_53 = define.stage {
        id = "netbattle-53",
        layout = {
            "normal normal poison poison normal normal",
            "normal normal poison poison normal normal",
            "normal normal poison poison normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_54 = define.stage {
        id = "netbattle-54",
        layout = {
            "normal normal poison poison normal normal",
            "normal normal poison poison normal normal",
            "normal normal poison poison normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_55 = define.stage {
        id = "netbattle-55",
        layout = {
            "normal poison poison poison poison normal",
            "normal poison poison poison poison normal",
            "normal poison poison poison poison normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 1, y = 2 }, { place = "navi", side = 1, x = 6, y = 2 } },
    },
    netbattle_56 = define.stage {
        id = "netbattle-56",
        layout = {
            "normal poison poison poison poison normal",
            "normal poison poison poison poison normal",
            "normal poison poison poison poison normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 6, y = 2 }, { place = "navi", side = 0, x = 1, y = 2 } },
    },
    netbattle_61 = define.stage {
        id = "netbattle-61",
        layout = {
            "normal normal holy holy normal normal",
            "normal normal holy holy normal normal",
            "normal normal holy holy normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_62 = define.stage {
        id = "netbattle-62",
        layout = {
            "normal normal holy holy normal normal",
            "normal normal holy holy normal normal",
            "normal normal holy holy normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_63 = define.stage {
        id = "netbattle-63",
        layout = {
            "normal normal normal normal normal normal",
            "holy holy holy holy holy holy",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_64 = define.stage {
        id = "netbattle-64",
        layout = {
            "normal normal normal normal normal normal",
            "holy holy holy holy holy holy",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_65 = define.stage {
        id = "netbattle-65",
        layout = {
            "normal holy normal normal holy normal",
            "normal holy normal normal holy normal",
            "normal holy normal normal holy normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_66 = define.stage {
        id = "netbattle-66",
        layout = {
            "normal holy normal normal holy normal",
            "normal holy normal normal holy normal",
            "normal holy normal normal holy normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_67 = define.stage {
        id = "netbattle-67",
        layout = {
            "cracked cracked cracked cracked cracked cracked",
            "cracked cracked cracked cracked cracked cracked",
            "cracked cracked cracked cracked cracked cracked",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_68 = define.stage {
        id = "netbattle-68",
        layout = {
            "cracked cracked cracked cracked cracked cracked",
            "cracked cracked cracked cracked cracked cracked",
            "cracked cracked cracked cracked cracked cracked",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_71 = define.stage {
        id = "netbattle-71",
        layout = {
            "ice ice ice ice ice ice",
            "ice missing ice ice missing ice",
            "ice ice ice ice ice ice",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 1, y = 2 }, { place = "navi", side = 1, x = 6, y = 2 } },
    },
    netbattle_72 = define.stage {
        id = "netbattle-72",
        layout = {
            "ice ice ice ice ice ice",
            "ice missing ice ice missing ice",
            "ice ice ice ice ice ice",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 6, y = 2 }, { place = "navi", side = 0, x = 1, y = 2 } },
    },
    netbattle_73 = define.stage {
        id = "netbattle-73",
        layout = {
            "poison poison poison poison poison poison",
            "poison normal poison poison normal poison",
            "poison poison poison poison poison poison",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_74 = define.stage {
        id = "netbattle-74",
        layout = {
            "poison poison poison poison poison poison",
            "poison normal poison poison normal poison",
            "poison poison poison poison poison poison",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_85 = define.stage {
        id = "netbattle-85",
        layout = {
            "missing missing missing missing missing missing",
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_86 = define.stage {
        id = "netbattle-86",
        layout = {
            "missing missing missing missing missing missing",
            "normal normal normal normal normal normal",
            "normal normal normal normal normal normal",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
    netbattle_87 = define.stage {
        id = "netbattle-87",
        layout = {
            "missing normal normal normal normal missing",
            "missing normal normal normal normal missing",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 0, x = 2, y = 2 }, { place = "navi", side = 1, x = 5, y = 2 } },
    },
    netbattle_88 = define.stage {
        id = "netbattle-88",
        layout = {
            "missing normal normal normal normal missing",
            "missing normal normal normal normal missing",
            "missing normal normal normal normal missing",
        },
        panel_pattern = 0x38,
        music = asset.sound("virus-battle"),
        background = asset.background("calendar-squares"),
        mode = 0,
        battle_number = 0,
        effects = 0x0000088C,
        actors = { { place = "navi", side = 1, x = 5, y = 2 }, { place = "navi", side = 0, x = 2, y = 2 } },
    },
}
