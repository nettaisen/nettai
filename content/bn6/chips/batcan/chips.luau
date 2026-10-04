--!strict
-- BatCan1-4: the navi raises its bat cannon and fires an unseen shot
-- (shot.luau) from the panel ahead on the 7th tick; BatCan4 fires three,
-- 16 ticks apart, the last staggering harder (the original's action 0x19,
-- `sub_80EC0E6`; docs/engine/shot-chips.md §6). The behavior is BN5's too
-- (@exelib/batcan, made in shot.luau with BN6's look).

local lockon = require("../../rules/lockon")
local shot = require("./shot")

-- A chip's action, for its shots: each one's hit modifier, in the order
-- they're fired (the original's chip parameters, or BatCan4's
-- `byte_80EC1BC`).
local batcan = shot.action

return {
    define.chip {
        id = "batcan1",
        codes = { "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 315, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 3 } },
    },
    define.chip {
        id = "batcan2",
        codes = { "S" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 316, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 3 } },
    },
    define.chip {
        id = "batcan3",
        codes = { "R" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 317, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 3 } },
    },
    define.chip {
        id = "batcan4",
        codes = { "Z" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 5, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage" },
        library = { number = 318, index = 1, sort = 0 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = batcan { shots = { 1, 1, 3 } },
    },
}
