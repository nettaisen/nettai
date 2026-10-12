--!strict
-- DarkNeo (EXE4 chip 0x140): the Program Advance of DrkLine, BugChain and Bass or BassAnly in that order (the recipe
-- table, the pointer list at 0x080212B0), a dark MegaMan's alone (its record's +0x15). The dimming chips' variant
-- 0x46 (0x080EA722): its controller (effect object #0x78, 0x080EA668) runs the navi chips' short course
-- (@exelib/navi_chips/controller: the user warps out, 11 ticks, the navi, 21 ticks after he left, the undim) for
-- its own Bass (./navi: he turns his panel to a hole, his user's image fires nine waves from it, and he blasts the two
-- columns past the middle of the field). The user is back at once as the undim starts, with no warp (0x080EA744).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local common = require("@exelib/navi_chips/controller")
local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips")
local recipe_bass = require("./bass").bass
local recipe_bassanly = require("./bassanly").bassanly
local recipe_bugchain = require("./bugchain").bugchain
local recipe_drkline = require("./drkline").drkline

-- 0x080EA744: the controller's side's navi is back. For a player's (the original's +0x0F 0, MegaMan's, or 0xF0 to
-- 0xFC, a navi's in the story) the controller first takes the navi's look itself, where the navi is, and stays
-- hidden: its sprite by its navi and soul (0x0800B90A), its shadow, its palette (0x0800BFFE), its side's facing, its
-- animation 0 asked for.
local function back(c: Object)
    local user = battle.player(c.alliance)
    if not user then
        error("DarkNeo's controller brings its user back, but its side has no navi (0x080EA744 reads through a null pointer)")
    end
    local player = user.actor_type == "player"
    if player then
        local sprite = c.sprite
        sprite:load_like(user)
        sprite.shadow = "ground"
        c.pos = user.pos
        sprite.palette = user.sprite.palette
        sprite:set_flip(bit32.bxor(c.alliance, c.flip))
        c.anim, c.anim_loaded = 0, 0xFF
    end
    c.visible = false
    if player then
        dimming.show_user(user)
    else
        dimming.show_actor(user)
    end
end

local CONTROLLER = common.make {
    id = "darkneo/controller",
    name = "DarkNeo",
    -- 0x080CB6E4: Bass for the controller, on its panel, with its element and the damage plus the bonus.
    spawn = function(c: Object, _user: Object, damage: number): Object?
        return navi.summon(c, c.panel_x, c.panel_y, c.element, damage)
    end,
    short = navi_chips.SHORT,
    -- Out as every navi chip's user (0x080E2D26); back at once.
    warp = function(c: Object, out: boolean)
        if out then
            navi_chips.warp(c.alliance, true)
        else
            back(c)
        end
    end,
}

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 250,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    program_advances = {
        { order = 2, sequence = { recipe_drkline, recipe_bugchain, recipe_bass } },
        { order = 3, sequence = { recipe_drkline, recipe_bugchain, recipe_bassanly } },
    },
    dimming = CONTROLLER.hook,
    megaman = "dark",
    auto_battle = { class = "into_reach", range = "ahead_then_column" },
}

return { darkneo = chip }
