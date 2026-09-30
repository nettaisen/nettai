--!strict
-- The bombs and seeds: action 0x12 (`sub_80EB628`). MiniBomb, EnergBom,
-- MegEnBom, BigBomb, FlshBom1-3, BlkBomb, BugBomb, the three seeds, the
-- LilBolrs and VDoll.
--
-- The navi winds up with the thing in hand (an attachment), throws it on
-- the tenth tick (it leaves from 4 pixels ahead, 48 up), and stands for 11
-- more, then 6 ticks of recovery. What is thrown is the chip's subtype's
-- (`off_80EB6F8`); what it holds is `byte_80EB738`'s. The damage word the
-- thrown object carries is the chip's damage and hit parameter plus the
-- Atk+ bonus.

local attachment = require("../../objects/attachment/attachment")
local bomb = require("../../objects/bomb/bomb")
local seed = require("../../objects/seed/seed")
local flash_bomb = require("../../objects/flash-bomb/flash_bomb")
-- WIP: the throwers not ported yet.
local function wip(name: string): (Object, Vec3, { number }, number) -> Object?
    return function(_: Object, _: Vec3, _: { number }, _: number): Object?
        error(name .. " is not ported yet")
    end
end
local black_bomb = { throw = wip("BlkBomb") }
local bug_bomb = { throw = wip("BugBomb") }
local lil_boiler = { throw = wip("LilBoiler") }
local voodoo_doll = { throw = wip("VDoll") }

export type State = {
    -- Ticks into the throw, then the recovery's countdown.
    timer: number,
}

local bombs = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local THROW, RECOVER = 0x0, 0x4

local ANIM_THROW = 6
local SOUND_THROW = 0xB2
-- The throw leaves on this tick, and the phase ends on this one.
local RELEASE_TICK, THROW_TICKS = 9, 0x15
local RECOVER_TICKS = 5
-- Where the thrown thing starts: ahead of the navi and up, 16.16.
local RELEASE_AHEAD, RELEASE_UP = 4 * 0x10000, 0x30 * 0x10000

-- A thrower: `user` throws from `pos` with the chip's parameters (as
-- bytes and as their word) and damage word.
type Thrower = (user: Object, pos: Vec3, params: { number }, damage: number) -> Object?

-- By subtype (`off_80EB6F8`): the thrower, and what the navi holds (the
-- attachment and its animation, `byte_80EB738`).
type Throw = { throw: Thrower, held: number, held_anim: number }
local THROWS: { [number]: Throw } = {
    [0x0] = { throw = bomb.throw, held = 0x04, held_anim = 0 },
    [0x1] = { throw = bomb.throw, held = 0x04, held_anim = 2 },
    [0x2] = { throw = bomb.throw, held = 0x0E, held_anim = 0 },
    [0x3] = { throw = lil_boiler.throw, held = 0x04, held_anim = 0 },
    [0x4] = { throw = bomb.throw, held = 0x2D, held_anim = 0 },
    [0x5] = { throw = bomb.throw, held = 0x04, held_anim = 0 },
    [0x6] = { throw = black_bomb.throw, held = 0x2D, held_anim = 0 },
    [0x7] = { throw = bug_bomb.throw, held = 0x04, held_anim = 4 },
    [0x8] = { throw = voodoo_doll.throw, held = 0x04, held_anim = 0 },
    [0x9] = { throw = seed.throw, held = 0x25, held_anim = 8 },
    [0xA] = { throw = bomb.throw, held = 0x04, held_anim = 6 },
    [0xB] = { throw = bomb.throw, held = 0x04, held_anim = 0 },
    [0xC] = { throw = seed.throw, held = 0x24, held_anim = 8 },
    [0xD] = { throw = seed.throw, held = 0x26, held_anim = 8 },
    [0xE] = { throw = flash_bomb.throw, held = 0x2E, held_anim = 2 },
    [0xF] = { throw = bomb.throw, held = 0x04, held_anim = 0 },
}

local FLASH_BOMB, BIG_BOMB = 0xE, 0xF

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

local function throw_of(me: Object): Throw
    local t = THROWS[me.variant]
    if not t then
        error(string.format("bomb subtype %#x reads past off_80EB6F8", me.variant))
    end
    return t
end

-- `sub_80EB644`: wind up, throw on the tenth tick, stand.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM_THROW)
        me:open_counter_window()
        local v = me.variant
        local t = throw_of(me)
        -- The held thing's palette: BigBomb's 3, a FlashBomb's by level.
        local palette_add = 0
        if v == BIG_BOMB then
            palette_add = 3
        elseif v == FLASH_BOMB then
            palette_add = int.u8(me:attack_param(1) * 3)
        end
        attachment.spawn_with(me, { t.held, t.held_anim, 0, palette_add }, "related")
        battle.play_sound(SOUND_THROW)
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == RELEASE_TICK then
        local p = me.pos
        local from = Vec3.new(p.x + me.facing * RELEASE_AHEAD, p.y, p.z + RELEASE_UP)
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        throw_of(me).throw(me, from, params, damage)
        -- Let go of what it held.
        me.related1 = nil
        me.overlay = nil
    end
    local t = s.timer + 1
    s.timer = t
    if t >= THROW_TICKS then
        set_phase(me, RECOVER)
    end
end

-- `sub_80EB758`: 6 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:exit_attack()
    end
end

-- `sub_80EB628`.
function bombs.update(me: Object, s: State)
    local step = me.step
    if step == THROW then
        throw(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("bomb phase %#x reads past off_80EB63C", step))
    end
end

return bombs
