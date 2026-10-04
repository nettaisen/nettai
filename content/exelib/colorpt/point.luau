--!strict
-- A point of ColorPt's and DblPoint's, as BN5 and BN6 share it (BN6's effect object #0x51, `sub_80E655C`; BN5's
-- #0x51, 0x080E65E8; BN6's docs/engine/dimming-chips.md §10.2): it gives its panel to the other side (the
-- stolen-area return brings it back), rises, and flies to its side's navi; there the chip at the hand's cursor, if it
-- does damage, gets its bonus (or the side's special bonus does). It tells its controller (the holder) when it is
-- gone.
--
-- Each game makes its own kind (`point.make`) with its look and its test for the special bonus: BN6's while a
-- special chip is in use (its player's AI+0xBB), BN5's with each side keeping its own gauge (battle flag 0x40,
-- 0x080E6746).


local point = {}

export type State = {
    -- Its controller, and its entry in the controller's `out` (the flag byte its RelatedObject1 points at).
    holder: Object?,
    slot: number,
    palette: number,
    bonus: number,
    -- Its navi's panel (Param3, Param4).
    target_x: number,
    target_y: number,
}

export type Look = {
    -- The kind's id.
    id: string,
    sprite: SpriteAsset,
    sounds: { appear: Sound, rise: Sound, arrive: Sound },
    -- Whether the bonus goes to the side's special bonus rather than to the chip at the hand's cursor.
    special: (side: number, navi: Object) -> boolean,
    -- The game's aiming helpers (its sine table's: lib/trajectory).
    trajectory: Trajectory,
}

export type Point = {
    kind: Kind,
    -- `sub_80E6720`: a point on panel (`x`, `y`) for `holder`, its entry `slot` in the holder's `out`.
    spawn: (holder: Object, x: number, y: number, spec: ColorPoint, slot: number) -> Object?,
}

-- A taken panel stays the other side's this long (`object_setPanelAllianceTimerLong`).
local STOLEN_TICKS = 0x708
-- Rising 4 pixels a tick to 16, then waiting 17 ticks; flying faster by 0.25 pixels a tick, every tick.
local RISE, HEIGHT = 0x4_0000, 0x10_0000
local ACCELERATION = 0x4000

-- The actions (the object's action byte).
local TAKE, LIFT, FLY = 0, 4, 8

-- Its controller is told it is gone (the flag byte cleared).
local function release(me: Object)
    local s = me.state :: State
    local holder = s.holder
    if holder then
        holder.state.out[s.slot] = false
    end
end

function point.make(look: Look): Point
    -- `sub_80E6600`: the panel goes to the other side; 19 ticks later it starts rising.
    local function take(me: Object)
        if me.phase_init == 0 then
            local x, y = me.panel_x, me.panel_y
            field.set_alliance(x, y, bit32.bxor(me.alliance, 1))
            field.set_column_timer(x, STOLEN_TICKS)
            me.timer = 0x13
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t > 0 then
            return
        end
        me.anim = 0
        battle.play_sound(look.sounds.rise)
        me:set_action(LIFT)
    end

    -- `sub_80E6644`: up to 16 pixels, then 17 ticks still.
    local function lift(me: Object)
        if me.phase_init == 0 then
            local p = me.pos
            local z = int.i32(p.z + RISE)
            me.pos = Vec3.new(p.x, p.y, z)
            if int.asr(z, 16) < 0x10 then
                return
            end
            me.pos = Vec3.new(p.x, p.y, HEIGHT)
            me.timer = 0x10
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t >= 0 then
            return
        end
        me:set_action(FLY)
    end

    -- `sub_80E667C`: toward the navi's panel, faster every tick; there, the bonus, and it is gone.
    local function fly(me: Object)
        local s = me.state :: State
        local tx, ty = field.center(s.target_x, s.target_y)
        local p = me.pos
        local angle = look.trajectory.angle(int.i32(tx - p.x), int.i32(ty - p.y))
        local vx, vy = look.trajectory.toward(angle, me.vel.x)
        me.pos = Vec3.new(int.i32(p.x + vx), int.i32(p.y + vy), p.z)
        me:set_panel_from_coordinates()
        if me.panel_x ~= s.target_x or me.panel_y ~= s.target_y then
            local v = me.vel
            me.vel = Vec3.new(int.i32(v.x + ACCELERATION), v.y, v.z)
            return
        end
        local side = me.alliance
        local navi = battle.player(side) :: Object
        if not look.special(side, navi) then
            local cursor = battle.hand_cursor(side)
            if battle.hand_chip_damages(side, cursor) then
                battle.add_hand_attack_bonus(side, cursor, s.bonus)
            end
        else
            battle.add_special_bonus(side, 0, s.bonus)
        end
        battle.play_sound(look.sounds.arrive)
        release(me)
        me:set_lifecycle("destroy")
    end

    -- `sub_80E6580`: in its look on its panel, aimed at its side's navi; with no navi, it is gone at once.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        -- (The next sprite update loads animation 1.)
        me.anim, me.anim_loaded = 1, 0xFF
        local navi = battle.player(me.alliance)
        if not navi then
            release(me)
            me:set_lifecycle("destroy")
            return
        end
        s.target_x, s.target_y = navi.panel_x, navi.panel_y
        me.visible = true
        sprite.palette = s.palette * 4
        me:set_coordinates_from_panel()
        battle.play_sound(look.sounds.appear)
        me:set_lifecycle("update")
        take(me)
    end

    local kind = define.kind {
        id = look.id,
        pool = "effect",
        state = {
            holder = "object",
            slot = "u8",
            palette = "u8",
            bonus = "u8",
            target_x = "u8",
            target_y = "u8",
        },
        -- `sub_80E655C`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                local a = me.action
                if a == TAKE then
                    take(me)
                elseif a == LIFT then
                    lift(me)
                elseif a == FLY then
                    fly(me)
                else
                    error(string.format("ColorPt's point: action %#x reads past off_80E65F4", a))
                end
            else
                me:free()
                return
            end
            me:update_sprite()
        end,
    }

    -- `sub_80E6720`: a point on panel (`x`, `y`) for `holder` (its side and facing), its entry `slot` in the
    -- holder's `out`. It runs while dimmed. (The game leaves the row in its damage word and register garbage in its
    -- element and Z's fraction; nothing reads them.)
    local function spawn(holder: Object, x: number, y: number, spec: ColorPoint, slot: number): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(y)
        o.alliance, o.flip = holder.alliance, holder.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.holder, s.slot = holder, slot
        s.palette, s.bonus = spec.palette, spec.bonus
        holder.state.out[slot] = true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return point
