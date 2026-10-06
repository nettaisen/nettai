--!strict
-- The countdown bomb, as EXE5 and EXE6 share it (EXE6's attack object #0x4B,
-- `sub_80CD8EC`; EXE5's)
-- the TimeBom chips set (./controller): a field obstacle that counts 3, 2,
-- 1 and blows up the whole enemy area (a hit over it and bursts over its
-- panels: objects/panel_bursts), unless it is broken first (a small puff,
-- no blast). It stays until it goes off; it has no lifetime.
--
-- What it is comes with its variant (a `CountdownVariant` record, a row of
-- the original's `byte_80CD8AC` by the first parameter): TimeBom1-3's and
-- TimeBom+'s are their chips' (./chips). The table has six more rows no
-- chip sets (2 to 7: one of its side's two field objects, hit by the other
-- side, that may appear under a body, with 3, 1, 5, 1, 10 and 1 HP; the
-- odd ones blow up when broken after a countdown of only 15 ticks, and
-- blink away otherwise): their branches are the variant's
-- `allows_bodies`, `bursts_when_broken` and `linked`.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions. Each game's bomb is a kind
-- of its look (`countdown.make_kind`): its sprite, sounds, the blast's
-- collision, how it looks absorbed, whether any removal just puffs (no
-- absorbing, no blinking out), its effects and its panel bursts' kind.

local panel_bursts = require("../panel_bursts")
local regions = require("../regions")

export type State = { variant: CountdownVariant }

local countdown = {}

local HIT_MOD = 3
-- Where it can stand once it is up: a solid panel on the field with
-- nothing on it (`allows_bodies`: bodies are fine).
local STANDS, OCCUPIED, OCCUPIED_BUT_BODIES = 0x10010, 0x0F88_0080, 0x0388_0080
-- The countdown: three steps of 60 ticks showing 3, 2 and 1 (the sprite
-- parts left hidden), then 30 more.
local STEP_TICKS, LAST_TICKS, SHORT_TICKS = 0x3C, 0x1E, 0xF
local SHOWS_3, SHOWS_2, SHOWS_1 = 0x3400_0000, 0x2C00_0000, 0x1C00_0000
-- Blinking away: 8 ticks.
local BLINK_TICKS = 8
-- The blast (`byte_80CDD30`): over the area of the side it hits for, no
-- hit spark, hit modifier 3; with bursts there every 2 ticks.
local BLAST_HIT_MOD = 3
local BURST_INTERVAL = 2
-- Its actions (`off_80CD9F4`).
local APPEAR, DESTROYED, COUNTDOWN, BLINK_AWAY = 0, 2, 8, 9


-- A countdown bomb kind (`id`) of `look`.
function countdown.make_kind(id: string, look: CountdownLook): Kind
    local SPRITE = look.sprite
    local PLACE_SOUND, TICK_SOUND, LAST_SOUND = look.sounds.place, look.sounds.tick, look.sounds.last
    local BLAST_SOUND, PUFF_SOUND = look.sounds.blast, look.sounds.puff

    local function variant(me: Object): CountdownVariant
        return (me.state :: State).variant
    end

    -- `sub_80CDCBC`: blow up (not once the battle is over): a hit over the
    -- enemy area, for the side opposite its panel's, bursts over it, a camera
    -- shake (two for TimeBom+), the sound.
    local function explode(me: Object)
        if battle.over() then
            return
        end
        me.visible = false
        me.collision:set_region(nil)
        local panel = field.panel(me.panel_x, me.panel_y)
        if not panel then
            error("sub_80CDCBC reads the panel of a countdown bomb off the field")
        end
        me.alliance = bit32.bxor(panel.alliance, 1)
        local area = regions.area[1 - me.alliance]
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = me.pos.z,
            region = area,
            -- None.
            hit_effect = nil,
            target = look.blast.hits,
            self_type = look.blast.is,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = BLAST_HIT_MOD,
        })
        panel_bursts.spawn(look.bursts, me, area, BURST_INTERVAL, 0, look.explosion)
        battle.shake_camera(1, 0x1E)
        battle.play_sound(BLAST_SOUND)
        if variant(me).strong_shake then
            -- TimeBom+ shakes it harder.
            battle.shake_camera(3, 0x3C)
        end
    end

    -- Gone: hidden, out of the field-object slots, destroyed next update.
    local function leave(me: Object)
        me.visible = false
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:set_lifecycle("destroy")
    end

    -- `sub_80CDA1C`: once the rising animation is over, stand if the panel
    -- allows (the countdown starts), else go at once.
    local function appear(me: Object)
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        local v = variant(me)
        local forbid = if v.allows_bodies then OCCUPIED_BUT_BODIES else OCCUPIED
        if not field.check(me.panel_x, me.panel_y, STANDS, forbid) then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x8_0000), look.puff)
            me:unreserve_panel(me.future_panel_x, me.future_panel_y)
            me.collision:set_region(nil)
            leave(me)
            return
        end
        me.anim = v.idle_anim
        me.timer = STEP_TICKS
        me.collision:set_region(regions.single)
        me.action, me.phase, me.phase_init = obstacle.action_byte(me, COUNTDOWN), 0, 0
        if v.linked then
            me:reserve_panel(me.future_panel_x, me.future_panel_y)
        end
        if v.bursts_when_broken then
            me.phase = 8
            me.timer = SHORT_TICKS
        end
    end

    -- One countdown step: when the timer runs out, show the next number.
    local function count(me: Object, shows: number, ticks: number, sound: Sound?, next_phase: number)
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        me.sprite.hidden_parts = shows
        me.timer = ticks
        if sound then
            battle.play_sound(sound)
        end
        me.phase = next_phase
    end

    -- `sub_80CDAD8` (phases `sub_80CDAFC`, `sub_80CDB2C`, `sub_80CDB50`,
    -- `sub_80CDB80`): 3, 2, 1, then blow up (or blink away).
    local function count_down(me: Object)
        local v = variant(me)
        local phase = me.phase
        if phase == 0 then
            -- (sub_80CDAFC tests for its first tick of TimeBom and TimeBom+
            -- and does nothing about it.)
            count(me, SHOWS_3, STEP_TICKS, TICK_SOUND, 4)
        elseif phase == 4 then
            count(me, SHOWS_2, STEP_TICKS, TICK_SOUND, 8)
        elseif phase == 8 then
            count(me, SHOWS_1, LAST_TICKS, if v.bursts_when_broken then nil else LAST_SOUND, 0xC)
        elseif phase == 0xC then
            local t = me.timer - 1
            me.timer = t
            if t ~= 0 then
                return
            end
            if v.bursts_when_broken then
                me.action, me.phase, me.phase_init = obstacle.action_byte(me, BLINK_AWAY), 0, 0
            else
                explode(me)
                me.action, me.phase, me.phase_init = DESTROYED, 0, 0
            end
        else
            error(string.format("the countdown bomb's countdown phase %#x reads past its table", phase))
        end
    end

    -- `sub_80CDBAC`: blink for 8 ticks, then go.
    local function blink_away(me: Object)
        if me.phase == 0 then
            me.phase = 1
            me.collision:set_region(nil)
            me.timer = BLINK_TICKS
        end
        me.visible = bit32.btest(me.timer, 2)
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            me.action, me.phase, me.phase_init = DESTROYED, 0, 0
        end
    end

    -- `sub_80CDBE8`: leave the field: a puff when broken (a blast for
    -- `bursts_when_broken` variants, once the battle isn't dimmed), or as a
    -- chip removed it.
    local function destroyed(me: Object)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me.collision:set_region(nil)
        local how = obstacle.removal(me)
        if how == "broken" then
            if me.hp == 0 and variant(me).bursts_when_broken then
                if battle.dimmed() then
                    return
                end
                explode(me)
            else
                battle.play_sound(PUFF_SOUND)
                battle.effect(me.pos, look.explosion)
            end
        elseif look.removal_puffs then
            -- (EXE5's 0x080CF21C: removed any way, a puff.)
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), look.puff)
        elseif how == "absorbed" then
            obstacle.fly_to_absorber(me, look.absorbed :: AbsorbedLook)
        else
            local blink = obstacle.blink_out(me)
            if blink == "blinking" then
                return
            elseif blink == "no" then
                local p = me.pos
                battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), look.puff)
            end
        end
        leave(me)
    end

    -- The bomb's action table (`off_80CD9F4`): its own actions and the
    -- framework's shared ones.
    local function act(me: Object, a: number)
        if a == APPEAR then
            appear(me)
        elseif a == 1 then
            obstacle.return_to_idle(me)
        elseif a == DESTROYED then
            destroyed(me)
        elseif a == 3 then
            obstacle.flinch(me)
        elseif a == 4 then
            obstacle.paralyzed(me)
        elseif a == 5 then
            obstacle.knocked_back(me)
        elseif a == 6 then
            obstacle.frozen(me)
        elseif a == 7 then
            obstacle.bubbled(me)
        elseif a == COUNTDOWN then
            count_down(me)
        elseif a == BLINK_AWAY then
            blink_away(me)
        else
            error(string.format("countdown bomb action %d reads past its table", a))
        end
    end

    -- `sub_80CD9CA`: at the end of the battle its HP drops to 0 (so it's
    -- broken); hits (a push keeps the damage), the reaction, its action.
    local function tick(me: Object)
        me.sprite.white = false
        if battle.over() then
            me.hp = 0
        end
        obstacle.take_hits(me, "keeps_damage")
        local a = obstacle.react(me)
        if a then
            act(me, a)
        end
        me:update_sprite()
        me.collision:present()
    end

    -- `sub_80CD91E`: on its panel, rising, with the variant's HP and a sound.
    local function init(me: Object)
        local v = variant(me)
        me:set_coordinates_from_panel()
        me.visible = true
        local sprite = me.sprite
        sprite:load(SPRITE)
        me.anim, me.anim_loaded = v.appear_anim, v.appear_anim
        sprite:set_animation(v.appear_anim)
        sprite.shadow = "ground"
        -- (sub_8002EAC sets a drawing flag of the sprite's.)
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        battle.play_sound(PLACE_SOUND)
        me.identity = v.identity
        me.hp, me.max_hp = v.hp, v.hp
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(v.collision.is, v.collision.hits, HIT_MOD)
        me.collision:set_region(nil)
        me.collision:present()
        me.lifecycle = "update"
        tick(me)
    end

    -- `sub_80CD8EC`.
    local function update(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end

    return new.kind {
        id = id,
        pool = "attack",
        state = { variant = "record:countdown-variant" },
        update = update,
    }
end

-- A bomb's variant, as a record the bomb keeps.
function countdown.variant(v: CountdownVariant): CountdownVariant
    return new.record("countdown-variant", v)
end

-- `sub_80CDC7C`: a bomb of `kind` and `variant` for `side` on (x, y) with
-- the damage word `damage`, registered in the side's field-object slots.
-- (Until its init places it, its position is the spawner's registers: Y,
-- the side and the bonus.)
function countdown.spawn_kind(
    kind: Kind,
    variant: CountdownVariant,
    x: number,
    y: number,
    side: number,
    damage: number,
    bonus: number
): Object?
    local o = battle.spawn(kind, Vec3.new(y, side, bonus))
    if not o then
        return nil
    end
    local s = o.state :: State
    s.variant = variant
    o.panel_x, o.panel_y = x, y
    o.alliance = side
    o:set_damage_word(damage)
    if variant.linked then
        -- The game links these variants to a slot at the spawner's r7 (the
        -- bomb's related1; its init and end clear the slot), which its only
        -- spawner (`sub_80E3290`) leaves as whatever the register held.
        error("sub_80CDC7C: a linked countdown bomb stores itself through a garbage pointer")
    end
    o.run_while_dimmed = true
    obstacle.register(o, side, variant.class)
    return o
end

return countdown
