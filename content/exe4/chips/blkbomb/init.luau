--!strict
-- BlkBomb (EXE4 chip 0x02E; its record at 0x080197EC + 0x2E * 0x2C): the throw, action 0x09's variant 6: the navi
-- holds the bomb in palette 4 (attachment row 0x2A, its animation 0: 0x080EB4EC) and lobs a black bomb (attack object
-- 0x47, 0x080D6120, thrown by 0x080D645E) at the panel three ahead, where it sits as one of the side's field objects
-- with 100 HP until a fire hit sets it off over the other side's area: the shared BlkBomb (@exelib/blkbomb/bomb, which
-- EXE5's grew from) in EXE4's look: no identity of its own, no tracking to release, a fire hit leaving its HP, its
-- destroyed action (0x080D6320) bursting it as it finishes; a touch destroys it with its HP as it is (0x0801416C); a
-- neutral body hit by everything, pushed with the damage kept (0x0801393E), the breaking spark, its landing's sound
-- (0xA2) and its burst's (0xEF).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local black_bomb = require("@exelib/blkbomb/bomb")
local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local panel_bursts = require("../objects/panel_bursts")
local rising_bubble = require("../objects/rising_bubble")
local sparks = require("../lib/sparks")
local trajectory = require("../lib/trajectory")

local SPRITE = asset.sprite("black-bomb")
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")

-- What the navi holds while throwing one: the bomb's sprite, palette 4 (attachment row 0x2A).
local HELD = attachment.look { sprite = asset.sprite("bomb"), palette = 4 }

local destroyed: (Object) -> ()

local LOOK: BlackBombLook = {
    destroyed = function(me: Object)
        destroyed(me)
    end,
    identity = nil,
    tracked = false,
    set_off_empties_hp = false,
    bursts_as_freed = false,
    crush = "destroys",
    sprite = SPRITE,
    spark = sparks.breaking,
    is = collision.neutral_body,
    target = collision.everything,
    burst_is = collision.attack,
    burst_target = collision.hits_navis,
    push = "keeps_damage",
    land_sound = asset.sound("metal-punch"),
    burst_sound = asset.sound("burst"),
    burst_effect = EXPLOSION,
    bubble = rising_bubble.kind,
    bursts = panel_bursts.kind,
    trajectory = trajectory,
}

-- 0x080D6368: it bursts (if a fire hit set it off), leaves the registry and its panel, and is freed at the next update.
local function finish(me: Object)
    black_bomb.burst(me, LOOK)
    obstacle.unregister(me)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
end

-- 0x080D6320: leaving. Hidden; broken (its HP 0), an explosion, and the bomb's sound unless a fire hit set it off,
-- carried to be thrown it goes at once; else not while dimmed.
destroyed = function(me: Object)
    me.visible = false
    if me.phase_init == 0 then
        me.phase_init = 4
        me.collision:set_region(nil)
        if me.hp == 0 then
            battle.effect(me.pos, EXPLOSION)
            if not (me.state :: { set_off: boolean }).set_off then
                battle.play_sound(SOUND_HIT_BOMB)
                if me:status("carried") then
                    finish(me)
                    return
                end
            end
        end
    end
    if battle.dimmed() then
        return
    end
    finish(me)
end

local KIND = black_bomb.make_kind("blkbomb/bomb", LOOK)

local blkbomb: Chip = {
    action = bombs.throw { id = "blkbomb/action", held = HELD, thrower = black_bomb.thrower(KIND) },
    codes = { "D", "H", "Z" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 3,
    mb = 55,
    damage = 210,
    hit_param = 148,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { blkbomb = blkbomb }
