--!strict
-- AirSpin's whirlwind top (attack object #0x9B in both games: EXE6's
-- `sub_80D88E0`, EXE5's 0x080D9EE4), made for a game's look (`top.make`).
-- Set down on its panel, it rolls forward at 4 pixels a tick toward the
-- next panel while that one is open (solid, nothing standing on it), at
-- most 4 panels, then stands 14 ticks and spins: it hits the ring around
-- its panel (a one-tick hit with the wind's gust) every 10 ticks, its
-- spins' times, and blows away. It breaks when its HP runs out or a body,
-- a neutral object or a type-2 attack touches it. It keeps running while
-- dimmed, but only acts outside dimming; it is its side's class-1 field
-- object.
--
-- The games part (`AirSpinTopLook`):
-- - EXE6's: a neutral body of 400 HP, set down only on a solid panel with no
--   body on it (else a puff, and gone); a wind hit spins it once more, and
--   a chip can remove it (absorbed, or blinking out).
-- - EXE5's: its side's own body of 50 HP, hit by the other side, set down
--   wherever it is sent; its gust hits without pushing; standing, and after
--   each spin, it draws RNG2 for a panel marker's style (9 or 11) and sets
--   that marker (battle flag 0x40's); its spins count down on Timer 2.

local regions = require("@exelib/regions")

local top = {}

export type State = {
    -- The X center of the panel it rolls toward (ExtraVars+0).
    target_x: number,
    -- Ticks between its hits (ExtraVars+4).
    interval: number,
    -- Spins left (the original's second parameter), and its palette
    -- (before the times 4: its first).
    spins: number,
    palette: number,
}

-- Rolling speed (16.16), panels it rolls at most.
local SPEED, MAX_PANELS = 0x4_0000, 4
-- Its field-object class.
local CLASS = 1
-- Panel flags: solid (0x10); where EXE6's can't be set down (bodies:
-- 0x3800000); what stops its rolling (bodies and neutral objects:
-- 0xF800000).
local SOLID, BODIES, BLOCKERS = 0x10, 0x03800000, 0x0F800000
-- Hits that break it at once: bodies, neutral objects, type 2.
local CRUSHING = 0x0F800002
-- Hits that make EXE6's spin once more: hit flag 0x800, or a wind hit
-- (secondary element 0x20).
local EXTRA_SPIN_HIT, WIND = 0x800, 0x20
-- Its hit modifier, and its gust's: the ring around its panel, no spark.
local HIT_MOD, HIT_MOD_SPIN = 0x03, 1
-- The marker styles EXE5's draws from (RNG2's bit 2).
local MARKER_STYLE, MARKER_STYLE_ALT = 9, 11
-- EXE5's marker over its panel as it stands.
local STAND_MARKER_TICKS = 0x14

-- The actions.
local ROLL, STAND, SPIN, BLOW_AWAY, REMOVED = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80D8C10` (EXE5's 0x080DA1AA): it can roll onto (x, y): on the field,
-- solid, nothing standing there.
local function open(x: number, y: number): boolean
    local f = field.flags(x, y)
    return f ~= 0 and not bit32.btest(f, BLOCKERS) and bit32.btest(f, SOLID)
end

-- `sub_80D8BBC`: the next panel forward is where it rolls, and its center
-- the X it rolls to.
local function aim(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    me.future_panel_x, me.future_panel_y = x, y
    local cx = field.center(x, y)
    local s = me.state :: State
    s.target_x = cx
end

-- `sub_800E6E8`: `center` lies past `from`, up to and including `to`.
local function crossed(to: number, from: number, center: number): boolean
    if to > from then
        return center > from and center <= to
    end
    return center > to and center <= from
end

-- `sub_800E24C`: the panel under the object's coordinates.
local function panel_under(me: Object): (number, number)
    local p = me.pos
    return int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28), int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
end

-- A whirlwind top of `look`'s game: its kind, `spawn` and AirSpin's `sender`.
function top.make(look: AirSpinTopLook): AirSpinTop
    -- A puff of wind at its position, raised by `lift` pixels.
    local function puff(me: Object, lift: number)
        battle.effect(me.pos + Vec3.px(0, 0, lift), look.puff)
    end

    -- EXE5's (0x080E9FD2's caller): RNG2's bit 2 picks the style of a marker
    -- over its panel for `ticks`.
    local function mark(me: Object, ticks: number)
        local marker = look.marker
        if marker then
            local style = if bit32.btest(battle.rng_positive(), 4) then MARKER_STYLE_ALT else MARKER_STYLE
            marker(me, me.panel_x, me.panel_y, ticks, style)
        end
    end

    -- Gone: out of the target tracking and the field-object registry, its
    -- region cleared, destroyed at the next update.
    local function leave(me: Object)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- ---- The actions -----------------------------------------------------------

    -- Stop rolling: onto its panel's center.
    local function stop(me: Object)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_action(STAND)
    end

    -- `sub_80D8A3C` (EXE5's 0x080D9FDC): roll toward the next panel while it
    -- is open; past its center, aim at the one after, for at most 4 panels.
    local function roll(me: Object)
        if me.phase_init == 0 then
            if not open(me.future_panel_x, me.future_panel_y) then
                return stop(me)
            end
            me.phase_init = 4
        end
        local s = me.state :: State
        local x = int.i32(me.pos.x + me.vel.x)
        if crossed(x, me.pos.x, s.target_x) then
            aim(me)
            if not open(me.future_panel_x, me.future_panel_y) then
                return stop(me)
            end
            local left = me.timer2 - 1
            me.timer2 = left
            if left <= 0 then
                return stop(me)
            end
        end
        -- (The panel under its coordinates stops it if it is open and not
        -- the one it is on, which rolling never makes.)
        local px, py = panel_under(me)
        if open(px, py) and me.panel_x ~= px then
            return stop(me)
        end
        local p = me.pos
        me.pos = Vec3.new(x, p.y, p.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
    end

    -- `sub_80D8AB6` (EXE5's 0x080DA056): stand 14 ticks (EXE5's marking its
    -- panel as it starts), then spin every 10.
    local function stand(me: Object)
        if me.phase_init == 0 then
            me.timer = 0x0F
            mark(me, STAND_MARKER_TICKS)
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            -- sub_80D8C06 (EXE5's 0x080DA18E)
            (me.state :: State).interval = 0x0A
            me:set_action(SPIN)
        end
    end

    -- `sub_80D8BD4` (EXE5's 0x080DA162): a hit around its panel, and the gust.
    local function gust(me: Object)
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            region = regions.ring,
            -- None.
            hit_effect = nil,
            target = look.gust_target,
            self_type = look.gust_self,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD_SPIN,
        })
        if look.gust_side then
            battle.effect(me.pos, look.gust, me.alliance, look.gust_palette)
        else
            battle.effect(me.pos, look.gust, nil, look.gust_palette)
        end
    end

    -- `sub_80D8ADC` (EXE5's 0x080DA092): spin, hitting at once and then every
    -- `interval` ticks, its spins' times (EXE5's counted on Timer 2, a marker
    -- over its panel after each).
    local function spin(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            if look.spins_on_timer2 then
                me.timer2 = s.spins
            end
            me.anim = 1
            me.timer = 0
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
        me.timer = s.interval
        gust(me)
        battle.play_sound(look.sounds.spin)
        local left
        if look.spins_on_timer2 then
            mark(me, s.interval + 3)
            left = me.timer2 - 1
            me.timer2 = left
        else
            left = s.spins - 1
            s.spins = left
        end
        if left <= 0 then
            me:set_action(BLOW_AWAY)
        end
    end

    -- `sub_80D8B18` (EXE5's 0x080DA0EA): a puff, and gone 3 ticks later.
    local function blow_away(me: Object)
        if me.phase_init == 0 then
            me.anim = 0
            puff(me, 0)
            me.timer = 4
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t <= 0 then
            leave(me)
        end
    end

    -- `sub_80D8B52` (EXE6's alone): removed by a chip: absorbed, it flies to
    -- the absorbing navi; blinking out, it blinks first; otherwise it goes
    -- in a puff.
    local function removed(me: Object)
        local absorbed = look.absorbed
        if absorbed and me:obstacle_flag("absorbed") then
            obstacle.fly_to_absorber(me, absorbed)
        else
            local b = obstacle.blink_out(me)
            if b == "blinking" then
                return
            elseif b == "no" then
                puff(me, 0x0C)
            end
        end
        leave(me)
    end

    local ACTIONS: { [number]: (Object) -> () } = {
        [ROLL] = roll,
        [STAND] = stand,
        [SPIN] = spin,
        [BLOW_AWAY] = blow_away,
    }
    if look.absorbed then
        ACTIONS[REMOVED] = removed
    end

    -- ---- The lifecycle -----------------------------------------------------------

    -- `sub_80D8988` (EXE5's 0x080D9F5C): take its hits; break, blow away off
    -- solid ground, or act (outside dimming, unless a chip just removed
    -- EXE6's).
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        local broken = me:take_damage(0) < 0 or bit32.btest(c.hit_flags, CRUSHING)
        if broken then
            battle.effect(me.pos, look.explosion)
            battle.play_sound(look.sounds.hit_bomb)
            leave(me)
        else
            local wind_hit = bit32.btest(c.hit_flags, EXTRA_SPIN_HIT) or bit32.btest(c.damage_elements, WIND)
            if look.extra_spins and wind_hit then
                local s = me.state :: State
                s.spins += 1
            end
            if not field.solid(me.panel_x, me.panel_y) then
                puff(me, 0)
                leave(me)
            else
                local run = not battle.dimmed()
                if look.absorbed and me:obstacle_flag("removed") and me.action ~= REMOVED then
                    me.action = REMOVED
                    me.phase = 0
                    me.phase_init = 0
                    run = true
                end
                if run then
                    local act = ACTIONS[me.action]
                    if act == nil then
                        error(string.format("air spin action %#x reads past its table", me.action))
                    end
                    act(me)
                end
            end
        end
        c:present()
    end

    -- `sub_80D8908` (EXE5's 0x080D9F0C): set down and roll (EXE6's only on a
    -- solid panel with no body; otherwise a puff, and gone).
    local function init(me: Object)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        if look.set_down_check then
            local f = field.flags(me.panel_x, me.panel_y)
            if not bit32.btest(f, SOLID) or bit32.btest(f, BODIES) then
                puff(me, 0x10)
                obstacle.unregister(me)
                me:free()
                return
            end
        end
        -- sub_8011504
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite:set_animation(0)
        sprite.shadow = "with_sprite"
        me.anim = 0
        me.anim_loaded = 0xFF
        if not me:create_collision() then
            -- (The game goes on without collision data and reads through a
            -- null pointer at the first update.)
            error("air spin: no collision slot for the whirlwind (sub_8011504)")
        end
        me:setup_collision(look.body, look.hit_by, HIT_MOD)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.visible = true
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        me.timer2 = MAX_PANELS
        me.identity = look.identity
        me.hp = look.hp
        me.max_hp = look.hp
        aim(me)
        sprite.palette = (me.state :: State).palette * 4
        battle.play_sound(look.sounds.set_down)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { target_x = "i32", interval = "i32", spins = "u8", palette = "u8" },
        -- `sub_80D88E0` (EXE5's 0x080D9EE4).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            if me.active and not battle.dimmed() then
                me:update_sprite()
            end
        end,
    }

    -- `sub_80D8B96` (EXE5's 0x080DA124): a whirlwind top for `owner` on
    -- (x, y) in `palette`, spinning `spins` times, with the damage word
    -- `damage`. It runs while dimmed and is its side's class-1 field object
    -- at once. Its position is the spawner's registers (the panel Y, the
    -- element and 0) until its init places it.
    local function spawn(
        owner: Object,
        x: number,
        y: number,
        element: number,
        palette: number,
        spins: number,
        damage: number
    ): Object?
        local o = battle.spawn(kind, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance, o.flip = owner.alliance, owner.flip
            o.related1 = owner
            o.run_while_dimmed = true
            local s = o.state :: State
            s.palette = palette
            s.spins = spins
            obstacle.register(o, o.alliance, CLASS)
        end
        return o
    end

    -- What AirSpin sends: a top in `palette` that spins `spins` times (the
    -- original's variant of AirSpin, with its first two parameters).
    local function sender(palette: number, spins: number): AirSpinSender
        return {
            palette = palette,
            spins = spins,
            send = function(user: Object, x: number, y: number, element: number, damage: number): Object?
                return spawn(user, x, y, element, palette, spins, damage)
            end,
        }
    end

    return { kind = kind, spawn = spawn, sender = sender }
end

return top
