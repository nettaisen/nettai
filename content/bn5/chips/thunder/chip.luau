--!strict
-- Thunder (BN5 chip 0x013): a thunder ball that homes in on the enemy for 5 panels and paralyzes what it hits:
-- BN6's Thunder (@bn6/chips/thunder, the ball BN6's attack object #0x2A, BN5's the same number; BN5's the same code
-- but for the ball's look, 0x080C9FA0, BN6's `sub_80C940C`): BN5's ball sprite (10-11, a fast or 12-panel ball's
-- 0c-53), animation 0, collision type row 0x0A (a 12-panel ball's, DarkThnd's, row 0x2B) with hit modifier 0
-- (BN6's an attack with 1), and its sound for every ball.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../../rules/collision")
local ball = require("@bn6/chips/thunder/ball")
local shoot = require("@bn6/chips/thunder/shoot")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local KIND = ball.make_kind("bn5:thunder/ball", {
    sprite = asset.sprite("bn5:thunder-ball"),
    small_anim = 0,
    big_sprite = asset.sprite("bn5:big-thunder-ball"),
    big_anim = 0,
    is = collision.thrown,
    dark_is = collision.collision_2b,
    hit_mod = 0,
    spark = sparks.elec,
    sound = asset.sound("bn5:erase-man-2"),
})

return define.chip {
    id = "bn5:thunder",
    codes = { "B", "L", "P", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 7,
    damage = 40,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = shoot.action { ball = { panels = 5, status = statuses.paralyze_90 }, kind = KIND },
}
