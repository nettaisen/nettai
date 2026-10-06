--!strict
-- A BlkBomb (the original's attack object #0x4A, `sub_80CD4EC`): the shared
-- bomb (@exelib/blkbomb/bomb), EXE6's kind and look: a neutral object whose
-- burst is an attack, pushed by a pushing hit with its damage kept, leaving
-- the field as `sub_80CD700` does.

local absorbed_obstacle = require("../../objects/absorbed_obstacle")
local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/blkbomb/bomb")
local effects = require("../../lib/effects")
local panel_bursts = require("../../objects/panel_bursts")
local rising_bubble = require("../../objects/rising_bubble")
local trajectory = require("../../lib/trajectory")

local black_bomb = {}

local SPRITE = asset.sprite("black-bomb")
-- Its identity: a field object, with the look something that takes its
-- place shows (DustMan's junk).
local IDENTITY = new.identity {
    id = "blkbomb",
    class = "field_object",
    version = 2,
    ai_index = 1,
    look = { sprite = asset.sprite("black-bomb"), anim = 0, palette = 0, shadow = true },
}
-- Its hit spark (0x0A).
local HIT_SPARK = new.spark { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 0 }
-- How it looks absorbed (the original's obstacle kind's sprite: its own).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")

-- What the navi holds while throwing one: the bomb's sprite, palette 4
-- (the original's attachment row 0x2D).
black_bomb.held = attachment.look { sprite = asset.sprite("bomb"), palette = 4 }

-- Gone from the field's bookkeeping, and freed at the next update.
black_bomb.finish = common.finish

-- `sub_80CD700`: leaving: a chip's removal (absorbed, blinking out, or a
-- puff), else broken (an explosion, with the bomb's sound unless a fire
-- hit set it off, which bursts as it goes); held while dimmed.
local function destroyed(me: Object)
    me.collision:set_region(nil)
    local how = obstacle.removal(me)
    if how ~= "broken" then
        if how == "absorbed" then
            obstacle.fly_to_absorber(me, ABSORBED)
        else
            local blink = obstacle.blink_out(me)
            if blink == "blinking" then
                return
            elseif blink == "no" then
                local p = me.pos
                battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
            end
        end
        black_bomb.finish(me)
        return
    end
    if me.phase_init == 0 then
        me.phase_init = 4
        me.visible = false
        if me.hp == 0 then
            battle.effect(me.pos, effects.explosion)
            if not (me.state :: { set_off: boolean }).set_off then
                battle.play_sound(SOUND_HIT_BOMB)
                black_bomb.finish(me)
                return
            end
        end
    end
    if battle.dimmed() then
        return
    end
    black_bomb.finish(me)
end

black_bomb.kind = common.make_kind("blkbomb/bomb", {
    destroyed = destroyed,
    identity = IDENTITY,
    sprite = SPRITE,
    spark = HIT_SPARK,
    is = collision.neutral_object,
    target = collision.everything,
    burst_is = collision.attack,
    burst_target = collision.hits_navis,
    push = "keeps_damage",
    land_sound = asset.sound("land"),
    burst_sound = asset.sound("burst"),
    burst_effect = effects.explosion,
    bubble = rising_bubble.kind,
    bursts = panel_bursts.kind,
    trajectory = trajectory,
})

-- `sub_80CD886`: a thrower for `throw.action` that throws a BlkBomb at the
-- panel three ahead of the user. (A game's own kind of it: `kind`.)
function black_bomb.thrower(kind: Kind?): Thrower
    return common.thrower(kind or black_bomb.kind)
end

return black_bomb
