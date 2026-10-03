--!strict
-- BN5's light and dark MegaMan (docs/design/bn5-map.md §6.1). A side's MegaMan is light or dark by his save's
-- light/dark value (NaviStats +0x44, read through 0x08012882): the system's setup, which a setup takes from the
-- save (bn5-compat from a recording's). Two of BN5's rules read it:
--
-- - **A dark MegaMan clears holy panels** (0x08017136, from the navi's intake every tick of the fight): when the
--   panel he stands on is holy and the value is 499 or less, the panel turns Normal. Any holy panel, whoever made
--   it.
-- - **The chips a MegaMan may use** (0x08010118, at the end of a chip use's preparation, 0x080100E6): a chip's
--   `megaman` field (its record's +0x15) asks for a light MegaMan (a value of 470 or more) or a dark one (less);
--   any navi but MegaMan (NaviStats +0x29 not 0) may use it. A chip he may not use becomes the invalid chip
--   (0x080100EC: chip 0x185, action 0x1A's variant 3 with no parameters), which shows a sparkle.
-- - **The dark chips** (flag 0x20; 0x08010030, before that check): one fizzles in a battle that bars them (battle
--   effect 0x100000), in a link battle for a light MegaMan (the chips' threshold), and for any navi but MegaMan
--   (or BN5's navi 23). Used, it marks its side (statistic 2 set to 1), leaves the navi's mood at 0 and ends his
--   anger, and costs him what the chip's entry of 0x0802C934 sets (a NaviCust bug of the battle: 0x0802C974's
--   routines; in battle flag 0x40 mode the HP drain one level more instead), the light/dark check following.

local invalid = require("../../chips/invalid/chip")
local megaman = require("../../navis/megaman/navi")

-- A navi's stats as the costs write them.
type Stats = Navi

-- The dark chips' costs (0x0802C974, by chip: BN5's chips 0xBB to 0xC6), the stats each sets.
local function custom_damage(stats: Stats)
    stats.custom_damage = bit32.band(stats.custom_damage + 100, 0xFFFF)
end
local function buster_blanks(stats: Stats)
    stats.buster_blanks = 4
end
local function step_bug(stats: Stats)
    stats.step_bug = 1
end
local function custom_drain(stats: Stats)
    stats.custom_drain = math.min(stats.custom_drain + 2, 7)
end
local COSTS: { [string]: (stats: Stats) -> () } = {
    ["bn5:darkcirc"] = custom_damage,
    ["bn5:drksword"] = buster_blanks,
    ["bn5:darkinvs"] = step_bug,
    ["bn5:darkplus"] = step_bug,
    ["bn5:drklance"] = function(stats: Stats)
        stats.hit_status = 2
    end,
    ["bn5:darkwide"] = buster_blanks,
    ["bn5:darkthnd"] = custom_drain,
    ["bn5:drkrecov"] = function(stats: Stats)
        stats.hp_drain = 7
    end,
    ["bn5:darkmetr"] = custom_drain,
    ["bn5:darkdril"] = function(stats: Stats)
        stats.panel_trail = 4
        stats.panel_trail_level = 4
    end,
    ["bn5:darktorn"] = custom_damage,
    ["bn5:drksonic"] = step_bug,
}

-- The side statistic a dark chip's use sets.
local DARK_CHIP_USED = 2

export type Setup = {
    -- The save's light/dark value (NaviStats +0x44).
    value: number,
}

-- A value at or below this is a dark MegaMan's for the holy panels (0x08017136: 0x1F3).
local CLEARS_HOLY = 499
-- A value at or above this is a light MegaMan's for the chips (0x08010118: 0xEB * 2).
local LIGHT_CHIPS = 470

-- 0x08010030: a dark chip's use; false when it fizzles.
local function use_dark_chip(side: number, navi: Object): boolean
    if battle.no_dark_chips() then
        return false
    end
    if battle.link() and (system.setup() :: Setup).value >= LIGHT_CHIPS then
        return false
    end
    -- (BN5's navi 23 may too; no other navi is in BN5's content.)
    if battle.navi(side).navi ~= megaman then
        return false
    end
    battle.set_side_stat(side, DARK_CHIP_USED, 1)
    -- 0x080127D6: the mood to 0 (unless it is 0); 0x08011A94: to 0x80 (unless it is 0), and anger ends.
    local stats = battle.navi(side)
    if stats.mood ~= 0 then
        stats.mood = 0
    end
    navi:end_anger()
    stats.mood = 0
    return true
end

-- 0x0802C934: the chip's cost, if it has one.
local function cost(side: number, chip: Chip)
    local f = COSTS[chip.id]
    if not f then
        return
    end
    local stats = battle.navi(side)
    if battle.per_player_gauges() then
        stats.hp_drain = math.min(stats.hp_drain + 1, 7)
        return
    end
    f(stats)
end

return define.system {
    id = "bn5:light-dark",
    setup = { value = "u16" },
    hooks = {
        -- 0x08017136: the holy panel under a dark MegaMan turns Normal.
        navi_intake = function(_side: number, navi: Object)
            if (system.setup() :: Setup).value > CLEARS_HOLY then
                return
            end
            local x, y = navi.panel_x, navi.panel_y
            local p = field.panel(x, y)
            if p and p.kind == "holy" then
                field.set_type(x, y, "normal")
            end
        end,
        -- 0x08010030: a dark chip's use, which may fizzle, and its cost; 0x08010118: a chip for the other kind of
        -- MegaMan becomes the invalid chip.
        chip_check = function(side: number, navi: Object, chip: Chip?): Chip?
            if chip == nil then
                return nil
            end
            if table.find(chip.flags or {}, "dark") and not use_dark_chip(side, navi) then
                return invalid
            end
            cost(side, chip)
            if battle.navi(side).navi ~= megaman then
                return nil
            end
            local wants = (chip :: any).megaman
            if wants == nil then
                return nil
            end
            local light = (system.setup() :: Setup).value >= LIGHT_CHIPS
            if (wants == "light") == light then
                return nil
            end
            return invalid
        end,
    },
}
