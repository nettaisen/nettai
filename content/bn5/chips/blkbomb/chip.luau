--!strict
-- BlkBomb (BN5 chip 0x02e): a black bomb lobbed three panels ahead that sits there until a fire hit sets it off
-- over the other side's area: BN6's BlkBomb (@bn6/chips/blkbomb, BN5's attack object #0x47, BN6's #0x4A; the same
-- code but for how it leaves, BN5's 0x080CE594, BN6's `sub_80CD700`), thrown by BN6's throw (the same held look,
-- BN5's attachment row 0x1A, BN6's 0x2D).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local black_bomb = require("@bn6/chips/blkbomb/bomb")
local effects = require("@bn6/lib/effects")
local throw = require("@bn6/lib/bombs/throw")

local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")

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

local KIND = black_bomb.make_kind("blkbomb/bomb", destroyed)

return define.chip {
    id = "blkbomb",
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
    action = throw.action { held = black_bomb.held, thrower = black_bomb.thrower(KIND) },
}
