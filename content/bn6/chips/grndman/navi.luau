--!strict
-- GroundMan (the original's actor object #0x17, `sub_80BBB98`), the navi of
-- the GrndMan chips, brought by the navi chip controller. He appears,
-- backs off a little, then drills along his row to the far column with his
-- drill (./drill), hitting each panel he enters. If he gets there the field
-- shakes and rocks (./rock) fall on the other side for 150 ticks, one every
-- 25 at random; blocked on the way (no floor, or the other side's body or an
-- object), he leaves at once. See docs/engine/chips.md §3.6.25.

local groundman_navi = require("../../navis/groundman/navi")
local navi_chips = require("../../lib/navi-chips/navi")
local collision = require("../../rules/collision")
local drill = require("./drill")
local panels = require("../../lib/panels")
local regions = require("../../lib/regions")
local rock = require("./rock")
local sparks = require("../../lib/sparks")

local grndman = {}

export type State = {
    -- His drill (ExtraVars[0]).
    drill: Object?,
    -- The last rock's panel (ExtraVars[1] and [2]).
    rock_x: number,
    rock_y: number,
    -- The controller waiting for him (its flag pointer sits in his
    -- collision data slot).
    controller: Object?,
}

local SPRITE = asset.sprite("groundman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_DRILL_OUT = asset.sound("drill-out")
local SOUND_DRILL = asset.sound("drill-spin")
local SOUND_ROCKFALL = asset.sound("rockfall")

-- What he wears (his overlay, `sub_8010F86`): the link navi's own parts.
local IDENTITY = groundman_navi.identity

-- How fast he backs off and drills (16.16).
local BACK_SPEED, DRILL_SPEED = 0x8000, 0x8_0000

-- A panel his drill goes on through, by side (`byte_80BBD50`: solid, and
-- no body or object of the other side's, nor a neutral one).
local DRILLABLE = { [0] = { 0x10, 0x0580_0000 }, [1] = { 0x10, 0x0A80_0000 } }

-- Where rocks fall, by side: first choice (`off_80BBEE0`: an enemy body on
-- the other side's area) and second (`byte_80BBEF4`: the other side's
-- area), { require, forbid }.
local ROCK_TARGETS = { [0] = { 0x0400_0020, 0 }, [1] = { 0x0800_0000, 0x20 } }
local ROCK_FALLBACK = { [0] = { 0x20, 0 }, [1] = { 0, 0x20 } }

-- His rocks: 10 ticks' warning, falling while dimmed, cracking the panel.
local ROCK = { warning = 0x0A, while_dimmed = true, cracks = true, silent = false }

-- His actions.
local APPEAR, BACK_OFF, DIG, ROCKFALL, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80BBBBC`: on his panel, on the ground (a word store: nothing of the
-- spawner's register is kept), with his parts.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me:add_parts(IDENTITY, 1)
    me:set_lifecycle("update")
end

-- Timer and Timer2 together (a word store).
local function set_timers(me: Object, ticks: number)
    me.timer = ticks
    me.timer2 = 0
end

-- `sub_80BBC2C`: he appears; 9 ticks; on, or straight to leaving if his
-- panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        set_timers(me, 9)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then BACK_OFF else LEAVE)
end

-- `sub_80BBC80`: 40 ticks; half a pixel back a tick while the timer is 25
-- down to 10; the drill comes out at 25.
local function back_off(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0
        me.vel = Vec3.new(-me.facing * BACK_SPEED, me.vel.y, me.vel.z)
        set_timers(me, 0x28)
        return
    end
    local t = me.timer
    if t <= 0x19 and t >= 0xA then
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    end
    local left = t - 1
    me.timer = left
    if left == 0x19 then
        me.anim = 0x11
        battle.play_sound(SOUND_DRILL_OUT)
        return
    end
    if left <= 0 then
        me:set_action(DIG)
    end
end

-- `sub_80BBE3A`: the hit on the panel he enters (`sub_80C53A6`: it runs
-- while dimmed).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.thrown_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80BBCD8`: drill to the far column (`sub_80BBE0E`: 8 pixels a tick,
-- 5 ticks a column; a BIOS division), hitting each panel he enters; there,
-- the rockfall; blocked, he leaves.
local function dig(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x12
        me.vel = Vec3.new(me.facing * DRILL_SPEED, me.vel.y, me.vel.z)
        local far = if me.alliance == 0 then 6 else 1
        me.timer = int.tdiv(math.abs(far - me.panel_x) * 0x28_0000, DRILL_SPEED)
        battle.play_sound(SOUND_DRILL)
        s.drill = drill.put_on(me)
        return
    end
    local old = me.panel_x
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    if me.panel_x ~= old then
        hit(me)
    end
    local ok = DRILLABLE[me.alliance]
    if not field.check(me.panel_x, me.panel_y, ok[1], ok[2]) then
        me:set_action(LEAVE)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(ROCKFALL)
    end
end

-- `sub_80BBE58`: a rock on the other side's area, on an enemy if there is
-- one, never on the last rock's panel (he stands there while he looks, so
-- the search leaves it out); one draw picks it.
local function drop_rock(me: Object)
    local s = me.state :: State
    local x, y = me.panel_x, me.panel_y
    me.panel_x, me.panel_y = s.rock_x, s.rock_y
    local want = ROCK_TARGETS[me.alliance]
    local found = panels.filtered_except(me, want[1], want[2])
    if #found == 0 then
        want = ROCK_FALLBACK[me.alliance]
        found = panels.filtered_except(me, want[1], want[2])
    end
    if #found > 0 then
        local i = int.tmod(battle.rng_positive(), #found)
        local rx, ry = panels.x(found[i + 1]), panels.y(found[i + 1])
        s.rock_x, s.rock_y = rx, ry
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        rock.drop(me, rx, ry, me.element, damage, ROCK, 0)
    end
    me.panel_x, me.panel_y = x, y
end

-- `sub_80BBD60`: 150 ticks of falling rocks, one every 25 from the first,
-- and the camera shaking.
local function rockfall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 0x96
        me.timer2 = 0
        local s = me.state :: State
        s.rock_x, s.rock_y = 0, 0
        battle.play_sound(SOUND_ROCKFALL)
        battle.shake_camera(2, me.timer)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(LEAVE)
        return
    end
    left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    drop_rock(me)
    me.timer2 = 0x19
end

-- `sub_80BBDAC`: the drill goes; 4 ticks (to -1), then he goes (his parts
-- with him) and the controller moves on.
local function leave(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 4
        set_timers(me, 4)
        drill.take_off((me.state :: State).drill)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me:remove_parts(IDENTITY)
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = { [APPEAR] = appear, [BACK_OFF] = back_off, [DIG] = dig, [ROCKFALL] = rockfall, [LEAVE] = leave }

grndman.kind = define.kind {
    id = "grndman/navi",
    pool = "actor",
    state = { drill = "object", rock_x = "u32", rock_y = "u32", controller = "object" },
    -- `sub_80BBB98`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("GroundMan action %#x reads past his table (off_80BBC18)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80BBDE8`: GroundMan for `user`, reporting to `controller` (the
-- GrndMan chips' navi hook, `off_802CD5C[10]`). His position is the
-- spawner's registers until his init places him.
function grndman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(grndman.kind, user, spec)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return grndman
