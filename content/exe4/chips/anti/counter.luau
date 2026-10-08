--!strict
-- EXE4's AntiDmg counter (navi action 0x38, 0x080ED972): the shared counter (@exelib/antidmg/counter, EXE6's code) with
-- EXE4's spec. Its vanish (0x080ED994) reserves the navi's panel and nothing more; it throws at a random one of the
-- other player navi's panels (0x0800FAF6), whatever the attack's variant, BodyGrd's diving shuriken (attack object
-- 0x5C, 0x080D8EFC: @exelib/bodygrd/shuriken, EXE6's code, with EXE4's look: 0c-2f, the crack's sound, effect row 2
-- as it shows, a thrown slash hitting navis; no more at its dive's start); its sounds 0xB1 as it vanishes and 0xAB as
-- it throws, its puff effect row 2. The rules start it by role (`actions.anti_damage_counter`, rules/roles.luau).

local collision = require("../../rules/collision")
local common = require("@exelib/antidmg/counter")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local shuriken = require("@exelib/bodygrd/shuriken")

local counter = {}

local PUFF = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 }

-- The shuriken's hit modifier (0x080D8F7C: 1, its own).
local HIT_MOD = 1

local SHURIKEN = shuriken.make_kind {
    id = "antidmg/shuriken",
    sprite = asset.sprite("shuriken"),
    land_sound = asset.sound("panel-crack"),
    puff = PUFF,
    self_type = collision.thrown_slash,
    target = collision.hits_navis,
}

-- 0x0800FAF6: one of the other player navi's panels (0x0800FB2C: its body's flag), picked by a draw; none, none.
local function target(me: Object): (number?, number?)
    local marked = regions.other_navi[me.alliance].field :: PanelCondition
    local list = panels.filtered(marked.require, marked.forbid)
    if #list == 0 then
        return nil, nil
    end
    local p = list[battle.rng_positive() % #list + 1]
    return panels.x(p), panels.y(p)
end

counter.action = common.action_at {
    id = "antidmg/counter",
    throw = function(me: Object, x: number, y: number, damage: number)
        -- 0x080D90A4: the shuriken onto (x, y) with the attack's element and damage word.
        shuriken.spawn(SHURIKEN, me, x, y, me.attack_element, HIT_MOD, damage)
    end,
    target = target,
    vanish_sound = asset.sound("trap-vanish"),
    throw_sound = asset.sound("boomerang"),
    puff = PUFF,
    keeps_move = true,
}

return counter
