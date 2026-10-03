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
--   (or BN5's navi 23). Used, it breaks a soul (0x08010070: the weakness request, 0x080122C8), marks its side
--   (statistic 2 set to 1), leaves the navi's mood at 0 and ends his
--   anger, and costs him what the chip's entry of 0x0802C934 sets (a NaviCust bug of the battle: 0x0802C974's
--   routines; in battle flag 0x40 mode the HP drain one level more instead), the light/dark check following.
-- - **The starting mood** (MegaMan's start, 0x08010EC8, where BN6's `sub_8013892` sets 0x80): by the value
--   (0x0801283A): under 470 0 (a dark MegaMan's, his face the fifth: 0x0801AF64), under 500 64, from 1000 190, else
--   the value / 20 + 103 (500: 0x80). A battle that holds the value (battle effect 0x20000) holds it at 500 from his
--   start.
-- - **His palette** (0x0800DD94, each tick: presentation): by the value's tier and the mood, the soul, the charge he
--   can't do, Hub Style (NaviStats +0x4C: the NaviCust's HubBatc, 0x08138214), the element; Dark MegaMan's (AI index
--   0x16) 1.
-- - **Hub Style's faces** (0x0801AF8E: the face plus 11): the base form's second set (`mugshot.variant`).

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

-- A value at or below this is a dark MegaMan's for the holy panels (0x08017136: 0x1F3).
local CLEARS_HOLY = 499
-- A value at or above this is a light MegaMan's for the chips (0x08010118: 0xEB * 2).
local LIGHT_CHIPS = 470

export type Setup = {
    -- The save's light/dark value (NaviStats +0x44): 0 to 1000; a fresh save's 500 (0x08010C00), the setup's
    -- default.
    value: number,
}

-- Dark MegaMan's AI index, and BN5's navi 23's (0x0800DE40: a computer navi of either draws in palette 1).
local DARK_MEGAMAN = 0x16
local NAVI_23 = 0x17

-- The side's light/dark value: the save's, or 500 in a battle that holds it (0x08010EDC, at MegaMan's start: nothing
-- reads it before).
local function value(): number
    if battle.light_dark_held() then
        return 500
    end
    return (system.setup() :: Setup).value
end

-- 0x0801283A: the starting mood by the value (its caller, 0x08010F1A, passes 0 for the Full Synchro it could ask).
local function starting_mood(v: number): number
    if v < 470 then
        return 0
    elseif v < 500 then
        return 64
    elseif v >= 1000 then
        return 190
    end
    return v // 20 + 103
end

-- 0x0800DE5C: the palette by the value's tier.
local function tier(v: number): number
    if v >= 1000 then
        return 4
    elseif v >= 500 then
        return 0
    elseif v >= 470 then
        return 3
    end
    return 2
end

-- 0x0800DD94 (BN6's `sub_801002C`): the palette of a navi of the player's kind.
local function palette(side: number, navi: Object): number?
    if navi.actor_type ~= "player" then
        -- A computer navi: Dark MegaMan's (and navi 23's) 1, any other 0.
        if navi.actor_type == "navi" and (navi.ai_index == DARK_MEGAMAN or navi.ai_index == NAVI_23) then
            return 1
        end
        return 0
    end
    local stats = battle.navi(side)
    if stats.navi ~= megaman then
        -- A link navi's (0x0800DA98, by BN5's navi numbers): BN5's content has none; the framework's.
        return nil
    end
    local in_soul = stats.form.kind ~= "base"
    local p = 2
    local hub = true
    if navi.ai_index == NAVI_23 then
        p = 2
    elseif navi:navi_state("no_charge") then
        p, hub = 1, false
    elseif in_soul then
        -- A soul's: 2 while its Chaos Unison charge is armed.
        return if navi.chaos_armed then 2 else 0
    elseif stats.mood == 0xFF then
        p = 4
    elseif stats.mood == 0 then
        p = if value() < LIGHT_CHIPS then 2 else 3
    else
        p = tier(value())
    end
    -- Hub Style (NaviStats +0x4C: the NaviCust's HubBatc in its fourth color, part 0x6F, 0x08138214).
    if hub and stats.hub_style then
        return p + 5 + 20
    end
    return p + (if in_soul then 0 else stats.element) * 5
end

-- 0x08010030: a dark chip's use; false when it fizzles.
local function use_dark_chip(side: number, navi: Object): boolean
    if battle.no_dark_chips() then
        return false
    end
    if battle.link() and value() >= LIGHT_CHIPS then
        return false
    end
    -- (BN5's navi 23 may too; no other navi is in BN5's content.)
    if battle.navi(side).navi ~= megaman then
        return false
    end
    -- In a soul, the soul breaks (the weakness request: 0x080122C8, the status section's `form_break`).
    if battle.navi(side).form.kind ~= "base" then
        navi:set_request("weakness_hit", true)
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
    -- A fresh save's value (0x08010C00 writes 500 to NaviStats +0x44): neither light nor dark.
    setup_defaults = { value = 500 },
    hooks = {
        -- Hub Style shows the base form's second set of faces (0x0801AF8E).
        round_setup = function(side: number)
            battle.set_face_variant(side, battle.navi(side).hub_style)
        end,
        -- 0x08010EC8: MegaMan's starting mood.
        starting_mood = function(_side: number): number?
            return starting_mood(value())
        end,
        -- 0x0800DD94: the navi's palette.
        navi_palette = palette,
        -- 0x08017136: the holy panel under a dark MegaMan turns Normal.
        navi_intake = function(_side: number, navi: Object)
            if value() > CLEARS_HOLY then
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
            local light = value() >= LIGHT_CHIPS
            if (wants == "light") == light then
                return nil
            end
            return invalid
        end,
    },
}
