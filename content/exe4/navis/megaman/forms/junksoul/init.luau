--!strict
-- JunkSoul (soul 10): EXE4's Double Soul with JunkMan, for a summon chip given up (0x08020008[9]).
-- MegaMan's sprite in it is 00-0a, his face picture 14 (0x08014B78), JunkMan's image junkman (08-0a). Its weapons
-- (0x080184F0's row 10): the buster on B, no A charge, Poltrgst charged (routine 0x61, ./charge). Its status reset
-- (0x0800E146) confuses the other side's navi: a hit of no damage on its panels with status 0x21, 720 ticks of
-- confusion. (What the souls share: forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").junksoul
local collision = require("../../../rules/collision")
local regions = require("@exelib/regions")
local soul = require("./soul")
local statuses = require("../../../rules/status")

local form: FormDef = {
    soul = { family = "summon", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("junkman")),
    sprite = asset.sprite("junksoul"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    -- 0x0800E146: the hit (attack 3 through 0x080CD81E, which runs while dimmed), anchored at his panel, its
    -- parameters by his side (0x0800E168: 0x1705FF85 or 0x1705FF84: the panels of the other side's player navi, no
    -- spark, at navis, row 0x17), no damage, no hit modifier, status 0x21.
    reset = function(navi: Object)
        local o = battle.hitbox(navi, {
            panel_x = navi.panel_x,
            panel_y = navi.panel_y,
            element = 0,
            z = 0,
            region = regions.other_navi[navi.alliance],
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.thrown_piercing_break,
            damage = 0,
            hit_mod = 0,
            status = statuses.confuse_720,
        })
        if o then
            o.run_while_dimmed = true
        end
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("junksoul") },
}

return { junksoul = form }
