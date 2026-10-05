--!strict
-- Sensor's turret (the original's attack object #0xA6, `sub_80DA050`;
-- docs/engine/dimming-chips.md §8.2): one of its side's field objects (an
-- obstacle) in front of its user for 720 ticks, aimed by its row (up from
-- the bottom row, down from the top one, straight from the middle). It
-- sends a scanner (scanner.luau) along its aim; when the scanner finds
-- one of the other side's bodies, it fires a laser down the line
-- (laser.luau), which stays out until the turret's lifetime ends.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local laser = require("./laser")
local scanner = require("./scanner")

local turret = {}

export type State = {
    -- Its look and HP (the chip's first two parameters).
    palette: number,
    hp: number,
    -- Its aim (Param3): 0 up, 1 straight, 2 down.
    aim: number,
    -- Its scanner and laser segments (ExtraVars+0, +4..+0x14), which
    -- register here and clear their entries when they end.
    scanner: Object?,
    segments: { Object? },
    -- Blinking out after its lifetime ran out (ExtraVars+0x18).
    blinking: boolean,
    -- Pushed: its scan starts over (ExtraVars+0x1C).
    restart: boolean,
}

local SPRITE = asset.sprite("sensor")
-- Its identity: a field object, with the look something that takes its
-- place shows (DustMan's junk).
local IDENTITY = define.identity {
    id = "sensor",
    class = "field_object",
    version = 5,
    ai_index = 1,
    look = { sprite = asset.sprite("sensor"), anim = 0, palette = 0, shadow = true },
}
local LIFETIME = 0x2D0
-- Its collision: another body of its side, reached by the other side.
local HIT_MOD = 3
-- How it looks absorbed (the original's obstacle kind's sprite: its own).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
local APPEAR_SOUND, BREAK_SOUND = asset.sound("appear"), asset.sound("hit-bomb-1")
local FIRE_SOUND = asset.sound("beast-over")
-- The aim by row (`dword_80DA104`), the aim's row step (`dword_80DA3E8`).
local AIM_BY_ROW: { [number]: number } = { [0] = 0, [1] = 2, [2] = 1, [3] = 0 }
local ROW_STEP: { [number]: number } = { [0] = -1, [1] = 0, [2] = 1 }
-- Its animations: standing (at its init), and by its aim: taking aim
-- (`dword_80DA1F4`), scanning (`dword_80DA244`) and firing
-- (`dword_80DA2D4`).
local ANIM_STAND, ANIM_IDLE = 0xA, 0
local ANIM_AIM: { [number]: number } = { [0] = 1, [1] = 0, [2] = 4 }
local ANIM_SCAN: { [number]: number } = { [0] = 2, [1] = 0, [2] = 5 }
local ANIM_FIRE: { [number]: number } = { [0] = 8, [1] = 7, [2] = 9 }
-- How long the laser stays (its lifetime, which then runs out).
local FIRE_TICKS = 0x78
local SEGMENTS = 5

-- The actions (the object's action byte).
local APPEAR, DESTROYED, PUSHED, SCAN, FIRE = 0, 2, 5, 8, 9

-- `sub_80DA424`: its scanner and laser segments end.
local function stop_parts(me: Object)
    local s = me.state :: State
    local sc = s.scanner
    if sc then
        scanner.stop(sc)
    end
    for i = 1, SEGMENTS do
        local seg = s.segments[i]
        if seg then
            laser.stop(seg)
        end
    end
end

-- `sub_80DA448`: pushed, its scan starts over (true if it did).
local function restarted(me: Object): boolean
    local s = me.state :: State
    if not s.restart then
        return false
    end
    s.restart = false
    me:set_action(SCAN)
    return true
end

-- Gone from the field's bookkeeping, hidden, and destroyed at the next
-- update.
local function finish(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80DA15C`: the appearing sound, then it scans.
local function appear(me: Object)
    me.timer2 = 0xA
    battle.play_sound(APPEAR_SOUND)
    me:set_action(SCAN)
end

-- `sub_80DA172`: after 40 ticks it takes aim, after 10 more turns, and
-- after 14 sends the scanner along its aim; once the scanner finds a
-- body, it fires. Not while dimmed.
local function scan(me: Object)
    if battle.dimmed() then
        return
    end
    restarted(me)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        me.timer2 = 0x28
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        me.timer2 = int.u16(me.timer2 - 1)
        if me.timer2 == 0 then
            me.anim = ANIM_IDLE
            me.timer2 = 0xA
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        me.timer2 = int.u16(me.timer2 - 1)
        if me.timer2 == 0 then
            me.anim = ANIM_AIM[s.aim]
            me.timer2 = 0xE
            me.phase, me.phase_init = 0xC, 0
        end
    elseif phase == 0xC then
        me.timer2 = int.u16(me.timer2 - 1)
        if me.timer2 == 0 then
            me.anim = ANIM_SCAN[s.aim]
            local x, y = me.panel_x + me.facing, me.panel_y + ROW_STEP[s.aim]
            scanner.spawn(me, x, y, s.aim, true)
            me.phase, me.phase_init = 0x10, 0
        end
    elseif phase == 0x10 then
        local sc = s.scanner
        if sc and scanner.found(sc) then
            scanner.stop(sc)
            me:set_action(FIRE)
        end
    else
        error(string.format("Sensor's turret: scan phase %#x reads past off_80DA190", phase))
    end
end

-- `sub_80DA266`: the laser down its aim, a segment a panel while the
-- line stays on the field (`sub_80DA290`); then it waits for its
-- lifetime to run out, or the battle's end. Not while dimmed, nor the
-- tick its scan starts over.
local function fire(me: Object)
    if battle.dimmed() or restarted(me) then
        return
    end
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        me.anim = ANIM_FIRE[s.aim]
        local dx, dy = me.facing, ROW_STEP[s.aim]
        local x, y = me.panel_x, me.panel_y
        local slot = 1
        -- `sub_80DA3C0`: until the line leaves columns 1 to 6 or rows 1 to
        -- 3 (registering in the slots after the turret's five when the line
        -- is longer, which its placement never lets it be).
        while true do
            x += dx
            if x == 0 or x > 6 then
                break
            end
            y += dy
            if y == 0 or y > 3 then
                break
            end
            if slot > SEGMENTS then
                error("Sensor's turret: a sixth laser segment writes past its ExtraVars")
            end
            laser.spawn(me, x, y, s.aim, slot)
            slot += 1
        end
        me.timer = FIRE_TICKS
        battle.play_sound(FIRE_SOUND)
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        if battle.over() then
            me:set_action(DESTROYED)
        end
    else
        error(string.format("Sensor's turret: fire phase %#x reads past off_80DA288", phase))
    end
end

-- `sub_80DA37A`: pushed, its scanner and laser end and its scan starts
-- over; then the push (`sub_8017CC0`).
local function pushed(me: Object)
    if me.drag_step == "start" then
        (me.state :: State).restart = true
        stop_parts(me)
    end
    obstacle.knocked_back(me)
end

-- `sub_80DA3EC`: blinking for 20 ticks (true when they're over).
local function blink(me: Object): boolean
    if me.phase_init == 0 then
        me.phase_init = 1
        me.timer = 0x14
    end
    me.visible = bit32.band(me.timer, 2) ~= 0
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t <= 0
end

-- `sub_80DA2EC`: leaving: a chip's removal (absorbed, blinking out, or a
-- puff); else, its lifetime having run out, it blinks out, and otherwise
-- it breaks (an explosion and its sound).
local function destroyed(me: Object)
    local how = obstacle.removal(me)
    local p = me.pos
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED)
    elseif how ~= "broken" then
        local b = obstacle.blink_out(me)
        if b == "blinking" then
            return
        elseif b == "no" then
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
    else
        local s = me.state :: State
        if not s.blinking and (me.timer ~= 0 or me.timer2 ~= 0) then
            battle.play_sound(BREAK_SOUND)
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x14_0000), effects.explosion)
        else
            s.blinking = true
            if not blink(me) then
                return
            end
        end
    end
    finish(me)
end

-- Its action table (`off_80DA134`): its own actions and the framework's
-- shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [APPEAR] = appear,
    [1] = obstacle.return_to_idle,
    [DESTROYED] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [PUSHED] = pushed,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [SCAN] = scan,
    [FIRE] = fire,
}

-- `sub_80DA108`.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("Sensor's turret: action %d reads past off_80DA134", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80DA07C`: on its panel, with its look, HP and lifetime, aimed by
-- its row.
local function init(me: Object)
    local s = me.state :: State
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = ANIM_STAND, 0
    sprite:set_animation(ANIM_STAND)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.identity = IDENTITY
    me.hp, me.max_hp = s.hp, s.hp
    me.timer = LIFETIME
    sprite.palette = s.palette
    s.aim = AIM_BY_ROW[me.panel_y]
    s.blinking = false
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

turret.kind = define.kind {
    id = "sensor/turret",
    pool = "attack",
    state = {
        palette = "u8",
        hp = "u8",
        aim = "u8",
        scanner = "object",
        segments = "object[5]",
        blinking = "bool",
        restart = "bool",
    },
    -- `sub_80DA050`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- `sub_80DA070`.
            stop_parts(me)
            me:destroy()
        end
    end,
}

-- `sub_80DA390`: a turret on panel (`x`, `y`) on `owner`'s side and
-- facing, with `element`, the damage word, and the chip's look and HP:
-- one of the side's field objects (the side's previous instrument, plane
-- or turret breaks). It acts while dimmed.
function turret.spawn(owner: Object, x: number, y: number, element: number, damage: number, palette: number, hp: number): Object?
    local o = battle.spawn(turret.kind, Vec3.zero)
    if not o then
        return nil
    end
    o:set_damage_word(damage)
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    obstacle.register(o, o.alliance, 1)
    local s = o.state :: State
    s.palette, s.hp = palette, hp
    return o
end

return turret
