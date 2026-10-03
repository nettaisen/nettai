--!strict
-- BlkBomb (BN5 chip 0x02e): a black bomb lobbed three panels ahead that sits there until a fire hit sets it off
-- over the other side's area: the shared BlkBomb (@common/blkbomb, BN5's attack object #0x47, BN6's #0x4A; the same
-- code but for how it leaves, BN5's 0x080CE594, BN6's `sub_80CD700`, and its hit intake, BN5's that of BN6's
-- `sub_801ADFA` where BN6's is `sub_801AD12`, which BN5 hasn't) with BN5's look (its spark, BN5's 0x0A, and
-- collision types; its identity, sprite, sounds, burst and what it hits BN6's), thrown by the shared throw (lib/bombs:
-- the same held look, BN5's attachment row 0x1A, BN6's 0x2D). (BN5's
-- action table has 8 rows to BN6's 10, no frozen or bubbled: its idle and its return are 7 and 6 where BN6's are 9
-- and 8, the same actions.)
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("../../objects/attachment/attachment")
local black_bomb = require("@common/blkbomb/bomb")
local bombs = require("../../lib/bombs")
local collision = require("../../rules/collision")
local effects = require("@bn6/lib/effects")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local SOUND_HIT_BOMB = asset.sound("bn5:hit-bomb-1")

-- 0x080CE594: leaving. Hidden at once; a chip's removal (BN5 has no absorbing and no blinking out: only the
-- removal) leaves a puff, and then, as when broken, an explosion if its HP is 0, with the bomb's sound unless a
-- fire hit set it off; carried to be thrown, it goes at once, else not while dimmed.
local function destroyed(me: Object)
    me.visible = false
    if me.phase_init == 0 then
        me.phase_init = 4
        me.collision:set_region(nil)
        if obstacle.removal(me) ~= "broken" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
        if me.hp == 0 then
            battle.effect(me.pos, effects.explosion)
            if not (me.state :: { set_off: boolean }).set_off then
                battle.play_sound(SOUND_HIT_BOMB)
                if me:status("carried") then
                    black_bomb.finish(me)
                    return
                end
            end
        end
    end
    if battle.dimmed() then
        return
    end
    black_bomb.finish(me)
end

local KIND = black_bomb.make_kind("bn5:blkbomb/bomb", {
    destroyed = destroyed,
    identity = define.identity {
        id = "bn5:blkbomb",
        class = "field_object",
        version = 2,
        ai_index = 1,
        look = { sprite = asset.sprite("bn6:black-bomb"), anim = 0, palette = 0, shadow = true },
    },
    sprite = asset.sprite("bn6:black-bomb"),
    spark = sparks.breaking,
    is = collision.neutral_object,
    target = collision.everything,
    burst_is = collision.attack,
    burst_target = collision.hits_navis,
    -- 0x08017984: any hit from the other side pushes it.
    push = "any_hit",
    land_sound = asset.sound("bn6:land"),
    burst_sound = asset.sound("bn6:burst"),
    burst_effect = effects.explosion,
    bubble = rising_bubble.kind,
    bursts = panel_bursts.kind,
    trajectory = trajectory,
})

-- What the navi holds while throwing one: the bomb's sprite, palette 4 (BN5's attachment row 0x1A).
local HELD = attachment.look { sprite = asset.sprite("bn6:bomb"), palette = 4 }

return define.chip {
    id = "bn5:blkbomb",
    codes = { "L", "R", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 3,
    mb = 55,
    damage = 210,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = HELD, thrower = black_bomb.thrower(KIND) },
}
