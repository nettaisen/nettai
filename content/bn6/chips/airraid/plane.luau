--!strict
-- AirRaid's plane (the original's attack object #0x75, `sub_80D34CC`;
-- docs/engine/dimming-chips.md §7.2): one of its side's field objects (an
-- obstacle) in front of its user with a propeller (propeller.luau). It
-- takes off, then, with a jet flame under it, drops a bomb
-- every 10 ticks: a panel strike (objects/panel-strike) on one of the
-- other side's bodies ahead of it in turn, now and then on a panel next
-- to it, or on the other side's back column with none ahead. It stays
-- until its lifetime runs out. (Its jet flame is LilBoiler's layer kind,
-- chips/lilbolr/layer.)
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local layer = require("../lilbolr/layer")
local panel_strike = require("../../objects/panel-strike/panel_strike")
local panels = require("../../lib/panels")
local propeller = require("./propeller")
local regions = require("../../lib/regions")

local plane = {}

export type State = {
    plane: AirRaidPlane,
    propeller: Object?,
    -- Its jet flame (ExtraVars+0xC).
    flame: Object?,
    -- The ticks to the next bomb (ExtraVars+4, a word).
    bomb_in: number,
    -- The next target in the list (ExtraVars+8).
    next_target: number,
}

local SPRITE = asset.sprite("bn6:air-raid-plane")
-- Its identity: a field object, with the look something that takes its
-- place shows (DustMan's junk).
local IDENTITY = define.identity {
    id = "bn6:airraid-plane",
    class = "field_object",
    version = 2,
    ai_index = 1,
    look = { sprite = asset.sprite("bn6:air-raid-plane"), anim = 0, palette = 0, shadow = true },
}
local FLAME = asset.sprite("bn6:jet-flame")
-- Its collision: another body of its side, reached by the other side.
local HIT_MOD = 3
-- How it looks absorbed (the original's obstacle kind 6: its own sprite).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
local APPEAR_SOUND, BREAK_SOUND = asset.sound("bn6:appear"), asset.sound("bn6:hit-bomb-1")
local TAKE_OFF_SOUND = asset.sound("bn6:take-off")
-- Taking off: 30 ticks, rising 0.625 pixels a tick.
local TAKE_OFF_TICKS, CLIMB = 0x1E, 0xA000
local BOMB_PERIOD = 10
-- The other side's bodies (by its side: `off_80D3834`).
local OTHER_BODY: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
-- The neighbour a bomb may stray to: the other side's panels, by side
-- (`byte_80D37E4`).
local STRAY: { [number]: { require: number, forbid: number } } = {
    [0] = { require = 0x0001_0020, forbid = 0 },
    [1] = { require = 0x0001_0000, forbid = 0x20 },
}
-- A bomb's strike: highlighted 6 ticks, its hits unmodified, no crack,
-- on any panel (its spawn parameters 0x20600).
local STRIKE: PanelStrike = { warning = 6, hit_mod = 0 }
-- With no neighbour to stray to, the game strikes column 0 of a row it
-- leaves in a register: the low byte of the panel table's address
-- (`ePanelData`) when the last neighbour it tried is on the field.
local PANEL_TABLE_LOW_BYTE = 0xE0

-- The actions (the object's action byte).
local APPEAR, DESTROYED, TAKE_OFF, BOMB = 0, 2, 8, 9

-- A plane.
function plane.plane(p: AirRaidPlane): AirRaidPlane
    return define.record("airraid-plane", p)
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

-- `sub_80D37F4`: the other side's bodies from the column ahead to the
-- field's far edge, column by column, rows 1 to 3 (packed x | y << 4).
local function targets(me: Object): { number }
    local list: { number } = {}
    local dir = me.facing
    local x = me.panel_x + dir
    if x == 0 then
        return list
    end
    while true do
        for y = 1, 3 do
            if field.check(x, y, OTHER_BODY[me.alliance], 0) then
                table.insert(list, x + y * 0x10)
            end
        end
        x += dir
        if x == 0 or x > 6 then
            return list
        end
    end
end

-- `sub_80D3770`: a bomb: the next of the targets, and one time in four a
-- random neighbour of it on the other side; with no target, the other
-- side's back column in its own row.
local function bomb(me: Object)
    local s = me.state :: State
    local list = targets(me)
    local x, y
    if #list == 0 then
        x, y = bit32.bxor(me.alliance, 1) * 5 + 1, me.panel_y
    else
        local i = s.next_target
        if i >= #list then
            i = 0
        end
        s.next_target = i + 1
        local p = list[i + 1]
        local draw = bit32.band(battle.rng_positive(), 7)
        x, y = bit32.band(p, 7), bit32.rshift(p, 4)
        if draw == 1 or draw == 5 then
            local dir = panels.side_direction(me.alliance)
            local cond = STRAY[me.alliance]
            local nx, ny = regions.random_panel(x, y, regions.neighbours, dir, cond.require, cond.forbid)
            if nx and ny then
                x, y = nx, ny
            else
                -- None: x 0, and the row the last test (the neighbour
                -- behind) left in a register.
                local last = x - dir
                y = if last >= 1 and last <= 6 then PANEL_TABLE_LOW_BYTE else y
                x = 0
            end
        end
    end
    panel_strike.spawn(me, x, y, 0, STRIKE, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), 0, false)
end

-- `sub_80D35E0`: the appearing sound, then it takes off.
local function appear(me: Object)
    me.anim = 0
    me.timer2 = 0xA
    battle.play_sound(APPEAR_SOUND)
    me:set_action(TAKE_OFF)
end

-- `loc_80D35FE`: rising for 30 ticks with its sound; not while dimmed.
local function take_off(me: Object)
    if battle.dimmed() then
        return
    end
    local phase = me.phase
    if phase == 0 then
        me.anim = 1
        me.timer2 = TAKE_OFF_TICKS
        battle.play_sound(TAKE_OFF_SOUND)
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, int.i32(p.z + CLIMB))
        me.timer2 = int.u16(me.timer2 - 1)
        if me.timer2 == 0 then
            me:set_action(BOMB)
        end
    else
        error(string.format("AirRaid's plane: take-off phase %#x reads past jt_80D3618", phase))
    end
end

-- `sub_80D3654`: its jet flame, then a bomb every 10 ticks while it has
-- any (`sub_80D369E`); the battle over, it goes. Not while dimmed.
local function bombing(me: Object)
    if battle.dimmed() then
        return
    end
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        me.anim = 2
        me.timer2 = s.plane.bombs
        s.bomb_in = 1
        s.flame = layer.spawn(me, { sprite = FLAME, anim = 1, z_offset = 0, behind = false, own_flip = false, palette = 0 })
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        if battle.over() then
            me:set_action(DESTROYED)
            return
        end
        -- Out of bombs, the count goes on down (a word), and nothing more
        -- happens.
        s.bomb_in = int.u32(s.bomb_in - 1)
        if s.bomb_in == 0 and me.timer2 ~= 0 then
            me.timer2 -= 1
            bomb(me)
            s.bomb_in = BOMB_PERIOD
        end
    else
        error(string.format("AirRaid's plane: bombing phase %#x reads past off_80D3670", phase))
    end
end

-- `sub_80D36CE`: leaving: a chip's removal (absorbed, blinking out, or a
-- puff), else broken (an explosion and its sound).
local function destroyed(me: Object)
    local how = obstacle.removal(me)
    if how == "broken" then
        battle.play_sound(BREAK_SOUND)
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x14_0000), effects.explosion)
    elseif how == "absorbed" then
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
    finish(me)
end

-- Its action table (`off_80D35B8`): its own actions and the framework's
-- shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [APPEAR] = appear,
    [1] = obstacle.return_to_idle,
    [DESTROYED] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [TAKE_OFF] = take_off,
    [BOMB] = bombing,
}

-- `sub_80D358C`.
local function tick(me: Object)
    me.visible = true
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    obstacle.tick_lifetime(me)
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("AirRaid's plane: action %d reads past off_80D35B8", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80D3500`: on its panel with its look, HP and lifetime, and its
-- propeller.
local function init(me: Object)
    local s = me.state :: State
    local p = s.plane
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.identity = IDENTITY
    me.hp, me.max_hp = p.hp, p.hp
    me.timer = p.lifetime
    sprite.palette = p.palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
    me.collision:present()
    s.propeller = propeller.spawn(me, false)
    me.lifecycle = "update"
    tick(me)
end

plane.kind = define.kind {
    id = "bn6:airraid/plane",
    pool = "attack",
    state = {
        plane = "record:airraid-plane",
        propeller = "object",
        flame = "object",
        bomb_in = "u32",
        next_target = "u32",
    },
    -- `sub_80D34CC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- `sub_80D34EC`: its propeller and flame go with it.
            local s = me.state :: State
            if s.propeller then
                propeller.stop(s.propeller)
            end
            layer.finish(s.flame)
            me:destroy()
        end
    end,
}

-- `sub_80D3742`: a plane on panel (`x`, `y`) on `owner`'s side and
-- facing, with the damage word: one of the side's field objects (the
-- side's previous instrument, plane or turret breaks). It acts while
-- dimmed.
function plane.spawn(owner: Object, x: number, y: number, p: AirRaidPlane, damage: number): Object?
    local o = battle.spawn(plane.kind, Vec3.zero)
    if not o then
        return nil
    end
    o:set_damage_word(damage)
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    obstacle.register(o, o.alliance, 1)
    local s = o.state :: State
    s.plane = p
    return o
end

return plane
