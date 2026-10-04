--!strict
-- VDoll's doll (attack object #0x7A: BN6's `sub_80D46B8`, BN5's 0x080D5618; the same code but where the look says):
-- a field object with 50 HP thrown three panels ahead in a 60-tick arc, which can hit in its last five ticks. Where
-- it lands the panel turns to poison. The first damage it takes after landing curses the other side (the game's
-- curse): a dimming that hits the other side with that damage. See docs/engine/shot-chips.md §14.2. A maker taking
-- a game's look.
--
-- An obstacle: its update runs the obstacle framework's steps (the `obstacle` service) around its own actions.

local regions = require("@exelib/regions")

local doll = {}

-- BN5's leaving (0x080D57F0): the curse's flash where no chip removed it (BN5's effect 0x24, 16 pixels up) and its
-- sound.
export type DollBn5Leaving = { flash: Effect, sound: Sound }

-- A game's doll: its kind's id, sheet and identity; how it looks absorbed (BN6's, which absorbs it); its collision
-- types and hit modifier from the start, and its spark; the ones it takes as it sets down (BN6's: an obstacle of no
-- side that anything can hit; BN5's has them from the start); the hits whose types break it at no damage (BN5's
-- 0x0C800002, 0x080D56D4); the poisoning's sound; the puff (effect 0x14); the game's aim; its curse; and BN5's
-- leaving (none: BN6's, `sub_80D48AE`).
export type DollLook = {
    id: string,
    sprite: SpriteAsset,
    identity: Identity,
    absorbed: AbsorbedLook?,
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    spark: Spark,
    set_down: { is: CollisionType, hits: CollisionType, hit_mod: number }?,
    breaking_hits: number?,
    poison_sound: Sound,
    puff: Effect,
    trajectory: Trajectory,
    curse: { spawn: (doll: Object, damage: number) -> Object?, telop: DimmingTelop },
    bn5_leaving: DollBn5Leaving?,
}

-- The doll made for a game.
export type Doll = { kind: Kind, thrower: () -> Thrower }

export type State = {
    -- The panel it aims at (Param3, Param4).
    target_x: number,
    target_y: number,
    -- Hurt (ExtraVars), the damage it took (ExtraVars+4), and still flying (ExtraVars+0xC).
    hit: boolean,
    damage_taken: number,
    flying: boolean,
}

local HP, LIFETIME = 0x32, 0x1E0
-- Its arc: 60 ticks to the target's center, falling 0x2000 a tick; it can hit in its last five.
local FLIGHT_TICKS, GRAVITY, HITS_FROM = 0x3C, -0x2000, 5
-- The curse's damage word: the damage taken, hit parameter 0xB2.
local CURSE_HIT_PARAM = 0xB2_0000
-- A puff 16 pixels up; the curse's own puff 12 up.
local PUFF_UP, CURSE_PUFF_UP = 0x10_0000, 0xC_0000

-- The actions (the object's action byte).
local DESTROYED, SET = 2, 8

function doll.make(look: DollLook): Doll
    -- The destroyed action's phases: BN6's start, leaving and cursing (`JumpTable80D489C`); BN5's leaving (its
    -- first tick a wait) and cursing (0x080D57E8).
    local LEAVING, CURSING = 0x4, 0x8
    if look.bn5_leaving then
        LEAVING, CURSING = 0x0, 0x4
    end

    -- (The HUD element it turns on with the curse and off as it goes, `sub_801DACC(0x40)` and `sub_801DA48(0x40)`,
    -- is drawn only, and so is `sprite_clearFinalPalette`.)

    -- `sub_80D47C0` (action 0): not while dimmed. The arc to the target; in its last five ticks it can hit; on the
    -- 61st it lands: on a solid panel, which turns to poison, or broken in a hole.
    local function fly(me: Object)
        local s = me.state :: State
        if battle.dimmed() then
            return
        end
        if me.phase_init == 0 then
            local cx, cy = field.center(s.target_x, s.target_y)
            me.vel = look.trajectory.in_ticks(me.pos, cx, cy, 0, FLIGHT_TICKS, GRAVITY)
            me.timer2 = FLIGHT_TICKS
            s.flying = true
            me.phase_init = 4
        end
        if me.timer2 < HITS_FROM then
            me.collision:set_region(regions.single)
        end
        local left = me.timer2 - 1
        me.timer2 = bit32.band(left, 0xFFFF)
        if left >= 0 then
            local p, v = me.pos, me.vel
            me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
            me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
            me:set_panel_from_coordinates()
            me:update_collision_panels()
            return
        end
        if not field.solid(me.panel_x, me.panel_y) then
            me.action = DESTROYED
            me.phase, me.phase_init = 0, 0
            return
        end
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me:update_collision_panels()
        field.set_type(me.panel_x, me.panel_y, "poison")
        battle.play_sound(look.poison_sound)
        s.flying = false
        me.action = SET
        me.phase, me.phase_init = 0, 0
    end

    -- Action 8 (BN6's `sub_80D4870`; BN5's 6, 0x080D57D0, nothing): set down, BN6's an obstacle of no side that
    -- anything can hit.
    local function set_down(me: Object)
        local set = look.set_down
        if set and me.phase == 0 then
            me.phase = 1
            me:reset_collision_types(set.is, set.hits, set.hit_mod)
        end
    end

    -- Gone quietly: out of the field's bookkeeping, and freed at the next update.
    local function go(me: Object)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me.collision:set_region(nil)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me.visible = false
        me:set_lifecycle("destroy")
    end

    -- An effect `up` pixels (16.16) over it.
    local function effect_up(me: Object, effect: Effect, up: number)
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + up)), effect)
    end

    -- A puff 16 pixels up, then gone.
    local function puff(me: Object)
        effect_up(me, look.puff, PUFF_UP)
        go(me)
    end

    -- The curse: hidden, the game's curse with the damage it took, the dimming.
    local function start_curse(me: Object)
        local s = me.state :: State
        local controller = look.curse.spawn(me, int.u32(CURSE_HIT_PARAM + s.damage_taken))
        dimming.start(me.alliance, true, controller, me, look.curse.telop)
        obstacle.release_tracking(me)
    end

    -- BN6's `sub_80D48AE`: how it leaves: blinking out; carried off; a chip's removal (absorbed, or a puff); or,
    -- hurt after landing, the curse.
    local function leave_bn6(me: Object)
        local s = me.state :: State
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "done" or me:status("carried") then
            go(me)
            return
        end
        local how = obstacle.removal(me)
        if how ~= "broken" then
            local absorbed = look.absorbed
            if how == "absorbed" and absorbed then
                obstacle.fly_to_absorber(me, absorbed)
                go(me)
            else
                puff(me)
            end
            return
        end
        if s.damage_taken == 0 then
            puff(me)
            return
        end
        if battle.dimmed() then
            return
        end
        if not s.hit or battle.over() then
            puff(me)
            return
        end
        if s.flying then
            go(me)
            return
        end
        me.phase, me.phase_init = CURSING, 0
        me.visible = false
        effect_up(me, look.puff, CURSE_PUFF_UP)
        start_curse(me)
    end

    -- BN5's 0x080D57F0: a tick's wait; then carried off, or not hurt, not hit, the battle over (a puff), or flying
    -- (gone); else the curse, with a puff 12 pixels up if a chip removed it, else BN5's look's flash 16 up and its
    -- sound. (No blinking out, no absorbing.)
    local function leave_bn5(me: Object)
        local s = me.state :: State
        local bn5 = look.bn5_leaving :: DollBn5Leaving
        if me.phase_init == 0 then
            me.phase_init = 4
            return
        end
        if me:status("carried") then
            go(me)
            return
        end
        if s.damage_taken == 0 then
            puff(me)
            return
        end
        if battle.dimmed() then
            return
        end
        if not s.hit or battle.over() then
            puff(me)
            return
        end
        if s.flying then
            go(me)
            return
        end
        me.phase, me.phase_init = CURSING, 0
        me.visible = false
        if obstacle.removal(me) ~= "broken" then
            effect_up(me, look.puff, CURSE_PUFF_UP)
        else
            effect_up(me, bn5.flash, PUFF_UP)
            battle.play_sound(bn5.sound)
        end
        start_curse(me)
    end

    -- BN6's `sub_80D49A0`, BN5's 0x080D58D0: hidden; once the dimming is over, gone.
    local function cursing(me: Object)
        me.visible = false
        if battle.dimmed() then
            return
        end
        obstacle.unregister(me)
        me.collision:set_region(nil)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me:set_lifecycle("destroy")
    end

    -- Action 2 (BN6's `sub_80D4888`, BN5's 0x080D57D4): by phase.
    local function destroyed(me: Object)
        local phase = me.phase
        if phase == LEAVING then
            if look.bn5_leaving then
                leave_bn5(me)
            else
                leave_bn6(me)
            end
        elseif phase == CURSING then
            cursing(me)
        elseif phase == 0 and not look.bn5_leaving then
            -- (BN6's start.)
            me.phase, me.phase_init = LEAVING, 0
        else
            error(string.format("VDoll destroyed phase %#x reads past its table", phase))
        end
    end

    -- Its action table (BN6's `off_80D479C`; BN5's 0x080D5704, BN6's numbers here): its own actions and the
    -- framework's shared ones.
    local ACTIONS: { [number]: (Object) -> () } = {
        [0] = fly,
        [1] = obstacle.return_to_idle,
        [2] = destroyed,
        [3] = obstacle.flinch,
        [4] = obstacle.paralyzed,
        [5] = obstacle.knocked_back,
        [6] = obstacle.frozen,
        [7] = obstacle.bubbled,
        [8] = set_down,
    }

    -- BN6's `sub_80D4754`, BN5's 0x080D56B4: any damage it takes this tick breaks it, and is kept for the curse
    -- (BN5's: a hit of the look's breaking types breaks it too, as damage 0).
    local function tick(me: Object)
        local s = me.state :: State
        me.visible = true
        obstacle.take_hits(me, "keeps_damage")
        me:hit_spark()
        obstacle.tick_lifetime(me)
        local c = me.collision
        local broke = look.breaking_hits ~= nil and bit32.btest(c.hit_flags, look.breaking_hits)
        local taken = if broke then 0 else c.final_damage
        if broke or taken ~= 0 then
            me.hp = 0
            s.hit = true
            s.damage_taken = taken
            c:set_region(nil)
        end
        local a = obstacle.react(me, "breaks")
        if a then
            local action = ACTIONS[a]
            if not action then
                error(string.format("VDoll action %d reads past its table", a))
            end
            action(me)
        end
        me:load_or_step_sprite()
        c:present()
    end

    -- BN6's `sub_80D46D8`, BN5's 0x080D5638: shown in its second animation, with 50 HP, of the look's collision
    -- types (its region off until it comes down).
    local function init(me: Object)
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = 1, 0
        sprite:set_animation(1)
        sprite:step()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_panel_from_coordinates()
        me.identity = look.identity
        me.hp, me.max_hp = HP, HP
        me.element = 0
        me.timer = LIFETIME
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.is, look.hits, look.hit_mod)
        me.collision:set_region(nil)
        me.collision:set_hit_effect(look.spark)
        me.collision:present()
        me.lifecycle = "update"
        tick(me)
    end

    local kind = define.kind {
        id = look.id,
        pool = "attack",
        state = {
            target_x = "u8",
            target_y = "u8",
            hit = "bool",
            damage_taken = "u32",
            flying = "bool",
        },
        -- BN6's `sub_80D46B8`, BN5's 0x080D5618.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }

    -- BN6's `sub_80D49F6`, BN5's 0x080D5926: a thrower for the throw action that throws a doll at the panel three
    -- ahead of the user (the original adds the column into a word over the row, so a column below 0 would borrow from
    -- it). It runs while dimmed and is one of its side's field objects (BN6's `sub_80D49CC`, BN5's 0x080D58FC).
    local function thrower(): Thrower
        return {
            throw = function(user: Object, pos: Vec3, damage: number): Object?
                local word = int.u32(user.panel_y * 0x100_0000 + (user.panel_x + 3 * user.facing) * 0x1_0000)
                local o = battle.spawn(kind, pos)
                if o then
                    local s = o.state :: State
                    s.target_x = bit32.band(bit32.rshift(word, 16), 0xFF)
                    s.target_y = bit32.rshift(word, 24)
                    o:set_damage_word(damage)
                    o.alliance, o.flip = user.alliance, user.flip
                    o.run_while_dimmed = true
                    obstacle.register(o, user.alliance, 1)
                end
                return o
            end,
        }
    end

    return { kind = kind, thrower = thrower }
end

return doll
