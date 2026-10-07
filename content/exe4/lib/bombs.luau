--!strict
-- EXE4's throws, the chips' action 0x09 (0x080EB3FE), and its thrown bomb (attack object #8, 0x080CE0A4). The throw
-- is EXE6's (@exelib/bombs/throw) in its phases and timing, with EXE4's own attachment and no held thing for a navi
-- of number 5 or 10 (NaviStats +0x23); the bomb is EXE6's code (@exelib/bombs/bomb), each variant a row of
-- 0x080CE088 (what it is and hits, its hit modifier and palette), its blast the region of 0x080CE270 and its
-- explosion the effect row of 0x080CE277. Each chip composes its throw from its variant's rows: what it holds
-- (0x080EB4EC: an attachment row and its animation) and what throws it (0x080EB4C4: the bomb's thrower, of the bomb
-- variant its parameter +0x10 names).

local attachment = require("../objects/attachment")
local bomb = require("@exelib/bombs/bomb")

local bombs = {}

local SPRITE = asset.sprite("bomb")

-- What a navi holds while throwing a bomb (attachment row 4, 0x080CD524: 0c-02 in palette 0; DarkBomb's row 0x11,
-- palette 3).
bombs.held = attachment.look { sprite = SPRITE, palette = 0 }
bombs.held_dark = attachment.look { sprite = SPRITE, palette = 3 }

-- A bomb variant (a row of 0x080CE088).
bombs.variant = bomb.variant

-- The effect rows the bomb shows (0x080E1698): a blast's explosion (row 0, DarkBomb's row 0x2B in palette 1), the
-- splash in a hole (row 1), the puff when the battle ends (row 0x15).
local EXPLOSION = asset.sprite("explosion")
bombs.effects = {
    explosion = new.effect { sprite = EXPLOSION, anim = 0, palette = 0 },
    dark_explosion = new.effect { sprite = EXPLOSION, anim = 0, palette = 1 },
    splash = new.effect { sprite = asset.sprite("small-puff"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
}

local made = bomb.make {
    id = "bomb",
    sprite = SPRITE,
    -- 0x080CE216.
    hit_sound = asset.sound("hit-bomb-1"),
    splash = bombs.effects.splash,
    explosion = bombs.effects.explosion,
    puff = bombs.effects.puff,
}

-- A thrower that throws a bomb of `variant` (0x080CE2B8), null-element.
bombs.thrower = made.thrower

export type BombThrow = {
    id: string,
    -- What the navi holds, and its animation.
    held: AttachmentLook,
    held_anim: number?,
    thrower: Thrower,
}

type State = { timer: number }

-- The phases, on the attack's step (0x080EB414's table).
local THROW, RECOVER = 0x0, 0x4
-- The throw leaves on this tick, and the phase ends on this one.
local RELEASE_TICK, THROW_TICKS = 9, 0x15
-- Where the thrown thing starts: 4 pixels right of the navi, whichever way it faces, and 48 up, 16.16 (0x080EB47A
-- works out the facing's 4 and then loads the X over it).
local RELEASE_RIGHT, RELEASE_UP = 4 * 0x10000, 0x30 * 0x10000
-- The navis that throw with nothing in hand (NaviStats +0x23: 5 and 10), and MegaMan's number.
local EMPTY_HANDED: { number } = { 5, 10 }
local NAVI_NUMBER = 0

local THROW_SOUND = asset.sound("minibomb-throw")

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- The chips' action 0x09 with a variant's rows.
function bombs.throw(spec: BombThrow): Action
    -- 0x080EB41C: wind up, throw on the tenth tick, stand.
    local function wind_up(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(6)
            me:open_counter_window()
            if not table.find(EMPTY_HANDED, NAVI_NUMBER) then
                attachment.attach(me, spec.held, "related", spec.held_anim or 0)
            end
            battle.play_sound(THROW_SOUND)
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        if s.timer == RELEASE_TICK then
            local p = me.pos
            local at = Vec3.new(p.x + RELEASE_RIGHT, p.y, p.z + RELEASE_UP)
            spec.thrower.throw(me, at, int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra))
            -- Let go of what it held.
            me.related1 = nil
        end
        local t = s.timer + 1
        s.timer = t
        if t >= THROW_TICKS then
            set_step(me, RECOVER)
        end
    end

    -- 0x080EB500: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 5
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- 0x080EB3FE.
    local function update(me: Object, s: State)
        local step = me.step
        if step == THROW then
            wind_up(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("throw phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = { timer = "u16" }, update = update }
end

return bombs
