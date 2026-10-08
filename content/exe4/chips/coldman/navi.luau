--!strict
-- ColdMan, the navi of EXE4's ColdMan chips (EXE4's actor object #0x56, 0x080CC04C: EXE4's own), and each chip's
-- controller, which brings him (effect object #0x7B: lib/navi_chips' short course). He appears on his user's panel
-- and, 11 ticks later (gone if it isn't solid), looks over the field's registered obstacles (the field-object
-- registry's first six slots, each side's; but those the field-taking chips leave, `obstacle.throwable`: 0x080CC35A,
-- 0x0800B3E8's kinds): each one with a free solid panel behind it as he faces (or his own panel there) he can reach,
-- and it turns to his side with his damage, encased in a bubble; each other one is encased in ice
-- (`obstacle.encase_in_bubble`, `encase_in_ice`: 0x0800B44C). Some found, the place sound and, 60 ticks later, he
-- visits the panels behind them, the farthest toward his enemy first (top row first in a column: 0x080CC388), and
-- punches each (by then the ice cube the bubble became, ../../objects/ice_cube) toward his enemy; last, on his own
-- panel, he forms an ice cube on the free solid panel ahead (gone if there's none) and, 15 ticks later, punches it.
-- His punch (0x080CC29C: `sub_80C53A6`, resolving while dimmed) is an aqua hit of 1 on the panel ahead, at navis, no
-- spark, hit modifier 0x21: the push that sends a cube sliding. 10 ticks after each punch (20 after the last
-- obstacle's) he goes on, and after his own he goes.

local collision = require("../../rules/collision")
local ice_cube = require("../../objects/ice_cube")
local navi_chips = require("../../lib/navi_chips")
local regions = require("@exelib/regions")

local coldman = {}

type State = {
    -- The controller waiting for him (the flag it gave him, his +0x4C).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- The panels he punches from (+0x60: y << 4 | x), his own first, and how many are left (+0x68).
    stops: { number },
    count: number,
}

local SPRITE = asset.sprite("coldman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_FOUND = asset.sound("place")
local SOUND_PUNCH = asset.sound("coldman-punch")
-- A puff 16 pixels above him as he goes (effect row 0x13).
local PUFF = new.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
local SOLID = 0x10
-- Bodies and blockers: a panel he stands on behind an obstacle, or forms his cube on, has none.
local OCCUPIED = 0x0F80_0000
-- His punch: aqua, 1 damage, hit modifier 0x21 (a push).
local AQUA, PUNCH_DAMAGE, PUNCH_HIT_MOD = 2, 1, 0x21
-- The animations: vanishing, appearing, winding up, punching, after a punch on his own panel.
local ANIM_VANISH, ANIM_APPEAR, ANIM_WIND_UP, ANIM_PUNCH, ANIM_AFTER = 3, 4, 5, 6, 7

local APPEAR, LOOK, ATTACK = 0x0, 0x4, 0x8

local function pack(x: number, y: number): number
    return bit32.band(bit32.lshift(y, 4) + x, 0xFF)
end

-- 0x080CC320: an obstacle (`o`); the panel behind it as he faces, his own or free solid ground, he stops on (true).
local function reaches(me: Object, o: Object, dir: number): boolean
    local s = me.state :: State
    local x, y = o.panel_x - dir, o.panel_y
    if not (x == me.panel_x and y == me.panel_y) and not field.check(x, y, SOLID, OCCUPIED) then
        return false
    end
    s.count += 1
    s.stops[s.count] = pack(x, y)
    return true
end

-- 0x080CC2E4: his own panel first; then each obstacle of the registry's six side slots, in order, he doesn't leave
-- (0x080CC35A): one he reaches turns to his side with his damage and is encased in a bubble, another in ice
-- (0x0800B44C).
local function survey(me: Object)
    local s = me.state :: State
    s.stops[1] = pack(me.panel_x, me.panel_y)
    s.count = 1
    local dir = 1 - 2 * me.alliance
    for side = 0, 1 do
        for _, o in field.objects(side) do
            if obstacle.throwable(o) then
                if reaches(me, o, dir) then
                    o.alliance = me.alliance
                    o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
                    obstacle.encase_in_bubble(o)
                else
                    obstacle.encase_in_ice(o)
                end
            end
        end
    end
end

-- 0x080CC388: the stops but his own, sorted so the last is the farthest toward his enemy (the greatest column for
-- side 0, the least for side 1), the top row first in a column; the stops are visited last first.
local function sort(me: Object)
    local s = me.state :: State
    local list = s.stops
    local r4 = s.count
    if r4 <= 2 then
        return
    end
    repeat
        local hi = r4
        local lo = hi - 1
        for _ = 1, r4 - 2 do
            local a, b = list[hi], list[lo]
            local ax, bx = bit32.band(a, 0xF), bit32.band(b, 0xF)
            local swap
            if me.alliance == 0 then
                swap = bx > ax or (bx == ax and b < a)
            else
                swap = bx < ax or (bx == ax and b < a)
            end
            if swap then
                list[lo], list[hi] = a, b
            end
            lo -= 1
        end
        r4 -= 1
    until r4 == 2
end

-- Action 0 (0x080CC110): 11 ticks (to -1); then, on a solid panel, he looks over the field (some obstacles found, the
-- place sound and his stops sorted); else he goes.
local function appear(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:set_lifecycle("destroy")
        return
    end
    survey(me)
    if (me.state :: State).count ~= 1 then
        battle.play_sound(SOUND_FOUND)
        sort(me)
    end
    me:set_action(LOOK)
end

-- Action 4 (0x080CC164): 2 ticks before his first stop with only his own, 61 with obstacles (their encasing's 60).
local function look(me: Object)
    me.timer = if (me.state :: State).count == 1 then 1 else 60
    me:set_action(ATTACK)
end

-- 0x080CC29C: his punch on the panel ahead, resolving while dimmed (`sub_80C53A6`).
local function punch(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + 1 - 2 * me.alliance,
        panel_y = me.panel_y,
        element = AQUA,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = PUNCH_DAMAGE,
        stamina = 0,
        hit_mod = PUNCH_HIT_MOD,
    })
    if box then
        box.run_while_dimmed = true
    end
    battle.play_sound(SOUND_PUNCH)
end

-- Action 8 (0x080CC17A), by phase: 0 (0x080CC19C) the wait out (to -1), then the next stop, the last first (none
-- left: he goes); there already, phase 0xC, else he vanishes (animation 3); 4 (0x080CC1E2) 3 ticks, then on the stop
-- he appears (animation 4); 8 (0x080CC206) 3 ticks; 0xC (0x080CC216) the punch: from a stop behind an obstacle at
-- once, from his own panel after forming a cube on the free solid panel ahead (else he goes) and 15 ticks; he winds
-- up (animation 5) and, as that ends, punches (animation 6); 20 ticks after the last obstacle's punch, 10 after the
-- others, phase 0.
local function attack(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        s.count -= 1
        if s.count < 0 then
            me:set_lifecycle("destroy")
            return
        end
        local stop = s.stops[s.count + 1]
        local x, y = bit32.band(stop, 7), bit32.rshift(stop, 4)
        me.future_panel_x, me.future_panel_y = x, y
        if x == me.panel_x and y == me.panel_y then
            me.phase, me.phase_init = 0xC, 0
        else
            me.anim = ANIM_VANISH
            me.timer = 3
            me.phase, me.phase_init = 4, 0
        end
    elseif phase == 4 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left ~= 0 then
            return
        end
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        me.anim = ANIM_APPEAR
        me.timer = 3
        me.phase, me.phase_init = 8, 0
    elseif phase == 8 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left == 0 then
            me.phase, me.phase_init = 0xC, 0
        end
    elseif phase == 0xC then
        local init = me.phase_init
        if init == 0 then
            if s.count ~= 0 then
                me.phase_init = 2
                me.anim = ANIM_WIND_UP
                return
            end
            me.phase_init = 1
            local x, y = me.panel_x + 1 - 2 * me.alliance, me.panel_y
            if not field.check(x, y, SOLID, OCCUPIED) then
                me:set_lifecycle("destroy")
                return
            end
            ice_cube.spawn(me, x, y, 0, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), 0)
            me.timer = 15
            if me.anim == ANIM_PUNCH then
                me.anim = ANIM_AFTER
            end
        elseif init == 1 then
            local left = me.timer - 1
            me.timer = int.u16(left)
            if left ~= 0 then
                return
            end
            me.phase_init = 2
            me.anim = ANIM_WIND_UP
        elseif bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim = ANIM_PUNCH
            punch(me)
            me.timer = if s.count == 1 then 20 else 10
            me.phase, me.phase_init = 0, 0
        end
    else
        error(string.format("ColdMan's attack's phase %#x reads past its table (0x080CC18C)", phase))
    end
end

-- 0x080CC0F2: his action (he never checks his panel again).
local function tick(me: Object)
    local action = me.action
    if action == APPEAR then
        appear(me)
    elseif action == LOOK then
        look(me)
    elseif action == ATTACK then
        attack(me)
    else
        error(string.format("ColdMan's action %#x reads past his table (0x080CC104)", action))
    end
end

-- 0x080CC090: his sprite (08-0E) with his shadow, shown, in animation 0, his chip's palette, facing his side's way, on
-- his panel; his sound; 10 ticks; then his first tick.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = s.palette
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND_APPEAR)
    me.timer = 10
    me:set_lifecycle("update")
    tick(me)
end

coldman.kind = new.kind {
    id = "coldman/navi",
    pool = "actor",
    state = { controller = "object", palette = "u8", stops = "u8[7]", count = "i8" },
    -- 0x080CC04C: by his state, then his sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080CC070: a puff 16 pixels above him; the controller moves on; he is freed.
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080CC2C0: ColdMan for the controller's user, with the damage plus the bonus, in the chip's palette. (His position
-- is the spawner's registers: the row, the element and 0.)
local function spawn(controller: Object, damage: number, palette: number): Object?
    local o = battle.spawn(coldman.kind, Vec3.new(controller.panel_y, controller.element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
    o.element = controller.element
    o.alliance = controller.alliance
    o:set_damage_word(damage)
    local s = o.state :: State
    s.controller = controller
    s.palette = palette
    return o
end

-- What each ColdMan chip states for its own controller: the controller's id (the original's effect object #0x7B for
-- all three: compat maps each to it) and his palette (0x080CC0BE by the chip's level: 0, the SP's 2, the DS's 3).
export type ChipSpec = { id: string, palette: number }

-- A ColdMan chip's controller: the short course (the user warps out, 11 ticks, ColdMan, 21 ticks after he left, the
-- user warps back in).
local function controller(chip: ChipSpec): NaviChipController
    return navi_chips.controller {
        id = chip.id,
        name = "ColdMan",
        spawn = function(c: Object, _user: Object, damage: number): Object?
            return spawn(c, damage, chip.palette)
        end,
        short = navi_chips.SHORT,
    }
end

return { controller = controller }
