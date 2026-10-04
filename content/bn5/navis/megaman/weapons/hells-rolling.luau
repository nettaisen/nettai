--!strict
-- The patch cards' HelzR (`megaman/hells-rolling`, B↓HelzR, Team Colonel's card 111): weapon routine 0x69
-- (0x0800FE36, calling 0x0800FB52, BN6's `sub_80121F8`'s place: BN5's own numbers) and action 0x47 (0x080EF030,
-- BN6's AirSpin, @exelib/airspin/action) in its variant 2 (0x080EF0C4's third entry, 0x080DEBB6: BN6's
-- `sub_80E025E`): a seeking whirlwind (attack object #0xC0, 0x080DE910: the shared one, @exelib/airspin/whirl, in
-- BN6's sheet and sound, an enemy navi invisible, submerged or bubbled unseen, 0x080DEC04) onto the first panel
-- ahead of the other side's, (Attack + 1) * 10 damage, hit parameter 0x94, Null.

local airspin = require("@exelib/airspin/action")
local collision = require("../../../rules/collision")
local whirl = require("@exelib/airspin/whirl")

local made = whirl.make {
    id = "bn5:megaman/hells-rolling/whirl",
    sprite = asset.sprite("bn5:bass-anly-shot"),
    sound = asset.sound("bn5:bass-anly-shot"),
    attack = collision.attack,
    hits = collision.hits_navis,
    hidden = { "invisible", "submerged", "bubbled" },
}

local SPIN = airspin.action { id = "bn5:megaman/hells-rolling/action", whirlwind = made.sender }

local HIT_PARAM = 0x94

-- 0x0800FB52: (Attack + 1) * 10 damage, hit parameter 0x94; Null, no Atk+ bonus; AirSpin's variant 2.
return define.weapon {
    id = "bn5:megaman/hells-rolling",
    -- The charge table's row 0x69, by Charge 0 to 5: past 4 it reads on into row 0x6A's first entry.
    charge_ticks = { 140, 130, 120, 110, 100, 60 },
    setup = function(navi: Object): Action
        navi.attack_damage = (battle.navi(navi.alliance).attack + 1) * 10
        navi.hit_param = HIT_PARAM
        navi.attack_element = 0
        navi.extra = 0
        return SPIN
    end,
}
