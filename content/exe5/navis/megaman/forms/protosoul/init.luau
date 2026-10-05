--!strict
-- ProtoSoul (soul 1, Team ProtoMan's): EXE5's Soul Unison with ProtoMan, for a sword chip given up. MegaMan's sprite
-- in it is 00-01 (0x0800DA58[1]); he changes by EXE5's soul change (rules/souls). Its weapons (0x0801CA1C's row 1,
-- loaded by 0x0800DCA0): the buster (routine 0), the charged slash (3, ./charge), Sword chips charged with A (5,
-- the A-charge's chips 0x0801090A: any Sword chip but a dimming or dark one; their damage doubled, 0x080103D0),
-- Chaos Unison's (AIData +0x11's routine 6, ./chaos: DrkSword), its B+Back shield (routine 4, ./back: EXE5's guard,
-- action 0x1F, as the Reflect program's).

local attach_points = require("@self/attach_points")
local back = require("@self/back")
local buster = require("../weapons/buster")
local chaos = require("@self/chaos")
local charge = require("@self/charge")
local charged_chip = require("../weapons/charged-chip")
local souls = require("../../../rules/souls")

return define.form {
    id = "protosoul",
    soul = { family = "sword" },
    change = souls.action,
    revert = souls.revert,
    sprite = asset.sprite("protosoul"),
    element = "null",
    buster_bonus = 0,
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        back_special = back.weapon,
        a_charge = charged_chip.weapon,
        chaos = chaos.weapon,
    },
    charged_chips = { { family = "sword", damaging = false, plain = true } },
    charged_bonus = { damage = 0xFF },
    palette = 0,
    -- Its face in the emotion window, beside its turns left (0x0801968E: pictures 5-10, the version's souls'); in its
    -- Chaos Unison look while the chaos charge is armed (0x080125F6: the palette 11 on, the pack's picture 0x10).
    mugshot = {
        normal = asset.mugshot("protosoul"),
        variant = { normal = asset.mugshot("protosoul-chaos") },
    },
    -- The buster arm he raises in it (lib/arm, 0x080EBABE): its animation, the soul's.
    buster_arm = { anim = 1 },
    -- His sword's arm (lib/swords, 0x080EC038): the blade's animation.
    blade_anim = 13,
    -- VarSwrd and NeoVari wait for their sequence while A is up (chips/varswrd, 0x080ED570).
    var_sword_waits = true,
    -- MegaMan's actor record, with his sprite in the soul's attach points (row 1 of category 0: what a chip's icon
    -- over him and his attachments sit by).
    identity = define.identity {
        class = "megaman",
        version = 0,
        actor_type = "player",
        ai_index = 0,
        aura_anim = 0,
        attach_points = attach_points,
    },
}
