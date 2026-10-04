--!strict
-- A LilBoiler (the original's attack object #0x93, `sub_80D75FC`): a field
-- object thrown three panels ahead. Its HP counts down from 4000 less its
-- damage, so the damage it will do (its max HP less its HP) grows with
-- every hit it takes, up to 999. After three hits that each lowered its
-- HP, or when its lifetime is about to run out, it erupts: its steam rises
-- (a layer, layer.luau), and 24 ticks later an aqua hit lands on the eight
-- panels around it. Crushed (a body, another obstacle, a breaking hit)
-- before that, it breaks. See docs/engine/shot-chips.md §14.1.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local layer = require("./layer")
local regions = require("@exelib/regions")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local boiler = {}

export type State = {
    -- Its level (Param1: 0, 2, 3 for LilBolr1-3): its palette and the
    -- damage its body deals.
    level: number,
    -- The panel it aims at (Param3, Param4).
    target_x: number,
    target_y: number,
    -- The angle it was thrown at (BattleObject+0x0C).
    angle: number,
    -- Hits that lowered its HP (ExtraVars), its HP after the last one
    -- (ExtraVars+0xC), erupting (ExtraVars+4) and its steam (ExtraVars+8).
    hits: number,
    last_hp: number,
    erupting: boolean,
    steam: Object?,
}

local SPRITE = asset.sprite("bn6:lil-boiler")
-- Its identity: a field object, with the look something that takes its
-- place shows (DustMan's junk).
local IDENTITY = define.identity {
    id = "bn6:lilbolr",
    class = "field_object",
    version = 0,
    ai_index = 1,
    look = { sprite = asset.sprite("bn6:lil-boiler"), anim = 0, palette = 0, shadow = true },
}
local MAX_HP, LIFETIME = 4000, 0xB4
-- Its HP never goes below this: it deals 999 at most.
local MIN_HP = 3001
-- Thrown at 0x2C000 a tick along the ground, falling 0x2800 a tick.
local SPEED, GRAVITY = 0x2C000, -0x2800
-- Its body's damage by level (`dword_80D7748`), its counter byte, its hit
-- modifier.
local BODY_DAMAGE: { [number]: number } = { [0] = 50, [1] = 50, [2] = 70, [3] = 90 }
local COUNTER, HIT_MOD = 0x1E, 3
local SOUND_LAND, SOUND_HIT_BOMB = asset.sound("bn6:land"), asset.sound("bn6:hit-bomb-1")
local SOUND_ERUPT, SOUND_STEAM = asset.sound("bn6:boiler-erupt"), asset.sound("bn6:boiler-steam")
-- The eruption: its animation and the steam's; the ticks to the hit; the
-- hit (on this tick of the count, aqua, hit modifier 3) and its sound's
-- tick; its effect over the panels (effect 0x32).
local ANIM_ERUPT, ANIM_STEAM = 5, 9
local ERUPT_TICKS, HIT_TICK, STEAM_TICK = 0x20, 8, 0x14
local AQUA, ERUPTION_HIT_MOD = 2, 3
local ERUPTION = define.effect { sprite = SPRITE, anim = 10, palette = 0 }
-- Its kind among absorbed obstacles.
-- How it looks absorbed (the original's obstacle kind's sprite: its own).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }
-- Its damage word's flags: doubled (0x8000), and the bits kept (0x7000).
local DOUBLED, FLAGS, AMOUNT = 0x8000, 0x7000, 0xFFF

-- The actions (the object's action byte).
local ERUPTING_OR_DESTROYED, SET = 2, 9
-- The eruption's phases.
local STEAM, RISE, BURST, DONE = 0x0, 0x4, 0x8, 0xC

-- Its HP number on the HUD (`sub_801DC7C(-16, 3)`): the damage it took,
-- which it erupts with. Drawn only; it goes as LilBoiler does
-- (`sub_801DD34`).
local HP_NUMBER: HpNumberSpec = { dx = -16, dy = 3, damage = true }

-- Gone from the field's bookkeeping, and freed at the next update.
local function finish(me: Object)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
    battle.hide_hp(me)
end

-- `sub_80D77D4` (actions 0 and 8): fly to the target panel; land on it,
-- or in a hole in a puff of smoke.
local function fly(me: Object)
    local s = me.state :: State
    local t = bit32.band(me.timer - 1, 0xFFFF)
    me.timer = t
    if t ~= 0 then
        local v = me.vel
        local vz = int.i32(v.z + GRAVITY)
        me.vel = Vec3.new(v.x, v.y, vz)
        local p = me.pos
        local moved = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + vz))
        if moved.z > 0 then
            me.pos = moved
            return
        end
    end
    local x, y = s.target_x, s.target_y
    me.panel_x, me.panel_y = x, y
    me.collision.panel_x, me.collision.panel_y = x, y
    local cx, cy = field.center(x, y)
    me.pos = Vec3.new(cx, cy, 0)
    if field.solid(x, y) then
        battle.play_sound(SOUND_LAND)
        battle.shake_camera(1, 0xF)
        me.collision:set_region(regions.single)
        me.run_while_dimmed = true
        me.action = SET
        me.phase, me.phase_init = 0, 0
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    rising_bubble.spawn(Vec3.new(p.x, p.y, 0), 0)
    battle.hide_hp(me)
    me.collision:set_region(nil)
    obstacle.unregister(me)
    obstacle.release_tracking(me)
    me.lifecycle = "destroy"
end

-- `sub_80D789C`: leaving: a chip's removal (absorbed, blinking out, or a
-- puff), else broken (an explosion when its HP ran out); held while
-- dimmed.
local function destroyed(me: Object)
    me.collision:set_region(nil)
    local how = obstacle.removal(me)
    if how ~= "broken" then
        if how == "absorbed" then
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
        return
    end
    if me.phase_init == 0 then
        me.phase_init = 4
        me.visible = false
        if me.hp == 0 then
            battle.effect(me.pos, effects.explosion)
            battle.play_sound(SOUND_HIT_BOMB)
            finish(me)
            return
        end
    end
    if battle.dimmed() then
        return
    end
    finish(me)
end

-- `sub_8109660(x, y, 0x10, 0)`: the eight panels around it highlighted
-- (drawn only), facing right whatever its side.
local function highlight(me: Object)
    for _, p in regions.ring.panels or {} do
        local x, y = me.panel_x + p[1], me.panel_y + p[2]
        if field.valid(x, y) then
            field.highlight(x, y)
        end
    end
end

-- `sub_80D7928`: the eruption, while not dimmed, by phase; its lifetime
-- held off all the while.
local function erupt(me: Object)
    local s = me.state :: State
    if not battle.dimmed() then
        local phase = me.phase
        if phase == STEAM then
            -- `sub_80D795C`: off its panels and out of the registry, its
            -- statuses cleared; the steam rises over it.
            me.collision:set_region(nil)
            me:clear_statuses()
            obstacle.release_tracking(me)
            battle.play_sound(SOUND_ERUPT)
            s.steam = layer.spawn(me, {
                sprite = SPRITE,
                anim = 0,
                z_offset = 0,
                behind = false,
                own_flip = false,
                palette = 0,
            })
            obstacle.unregister(me)
            me:reserve_panel(me.future_panel_x, me.future_panel_y)
            me.phase, me.phase_init = RISE, 0
        elseif phase == RISE then
            -- `sub_80D7996`.
            me.anim = ANIM_ERUPT
            local steam = s.steam
            if steam then
                steam.anim = ANIM_STEAM
            end
            me.timer2 = ERUPT_TICKS
            me.phase, me.phase_init = BURST, 0
        elseif phase == BURST then
            -- `sub_80D79AC`: on the count's 8 the hit over the eight panels
            -- around it and their effects, and the steam goes; at 0x14 a
            -- sound; then done. The panels show while bit 2 of its
            -- lifetime is clear.
            local t = me.timer2
            if t == HIT_TICK then
                local word = int.u32(me.damage + me.stamina * 0x10000)
                battle.hitbox(me, {
                    panel_x = me.panel_x,
                    panel_y = me.panel_y,
                    element = AQUA,
                    z = 0,
                    region = regions.ring,
                    hit_effect = nil,
                    target = collision.hits_navis,
                    self_type = collision.attack,
                    damage = bit32.band(word, 0xFFFF),
                    stamina = bit32.rshift(word, 16),
                    hit_mod = ERUPTION_HIT_MOD,
                })
                battle.region_effects(me.panel_x, me.panel_y, regions.ring, bit32.bxor(me.alliance, me.flip), ERUPTION, 0)
                layer.finish(s.steam)
            end
            if t == STEAM_TICK then
                battle.play_sound(SOUND_STEAM)
            end
            local left = t - 1
            me.timer2 = bit32.band(left, 0xFFFF)
            if left < 0 then
                me.phase, me.phase_init = DONE, 0
            end
            if bit32.band(me.timer, 4) == 0 then
                highlight(me)
            end
        elseif phase == DONE then
            -- `sub_80D7A0E`.
            me:unreserve_panel(me.future_panel_x, me.future_panel_y)
            me.visible = false
            me:set_lifecycle("destroy")
            battle.hide_hp(me)
        else
            error(string.format("LilBoiler eruption phase %#x reads past off_80D794C", phase))
        end
    end
    me.timer = bit32.bor(me.timer, 0x20)
end

-- `sub_80D787C`: its HP number goes; erupting, or (the battle over, or
-- not erupting) leaving.
local function erupting_or_destroyed(me: Object)
    battle.hide_hp(me)
    if battle.over() or not (me.state :: State).erupting then
        destroyed(me)
    else
        erupt(me)
    end
end

-- `sub_80D7878`: set down, nothing to do.
local function idle(_me: Object) end

-- Its action table (`off_80D7798`): its own actions and the framework's
-- shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = fly,
    [1] = obstacle.return_to_idle,
    [2] = erupting_or_destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = fly,
    [9] = idle,
}

-- `sub_80D7A2A`: it erupts when its lifetime is about to end, or on the
-- third hit that lowered its HP; its damage is then what it lost.
local function check_eruption(me: Object)
    local s = me.state :: State
    if me.action == ERUPTING_OR_DESTROYED then
        return
    end
    if me.timer ~= 1 then
        if s.erupting or me.collision.hit_flags == 0 then
            return
        end
        local last = s.last_hp
        s.last_hp = me.hp
        if last == me.hp then
            return
        end
        s.hits += 1
        if s.hits < 3 then
            return
        end
    end
    s.erupting = true
    me.damage = bit32.bor(int.u16(me.max_hp - me.hp), bit32.band(me.damage, FLAGS))
    me.action = ERUPTING_OR_DESTROYED
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D774C`.
local function tick(me: Object)
    local s = me.state :: State
    me.visible = true
    obstacle.take_hits(me, "keeps_damage")
    me:hit_spark()
    obstacle.tick_lifetime(me)
    -- `sub_801B878`: "breaks", but a crushing touch is as any hit while it
    -- erupts.
    local a = obstacle.react(me, if s.erupting then "ignores" else "breaks")
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("LilBoiler action %d reads past off_80D7798", a))
        end
        action(me)
    end
    if me.hp <= MIN_HP then
        me.hp = MIN_HP
    end
    check_eruption(me)
    if s.erupting then
        local hp = me.max_hp - bit32.band(me.damage, AMOUNT)
        me.hp = int.u16(hp)
        s.last_hp = int.u32(hp)
    end
    me:update_sprite()
    me.collision:present()
end

-- `sub_80D761C`: aim at the target panel; its HP from its damage.
local function init(me: Object)
    local s = me.state :: State
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.shadow = "ground"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = 3 * s.level
    me.identity = IDENTITY
    local x, y = s.target_x, s.target_y
    me.future_panel_x, me.future_panel_y = x, y
    local tx, ty = field.center(x, y)
    local p = me.pos
    s.angle = trajectory.angle(int.i32(tx - p.x), int.i32(ty - p.y))
    local vx, vy = trajectory.toward(s.angle, SPEED)
    me.vel = Vec3.new(vx, vy, me.vel.z)
    local vz, ticks = trajectory.at_speed(me.pos, vy, tx, ty, 0, SPEED, GRAVITY)
    me.vel = Vec3.new(vx, vy, vz)
    me.timer = ticks
    if ticks == 0 then
        me.pos = Vec3.new(tx, ty, 0)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:destroy()
        return
    end
    local damage = me.damage
    if bit32.band(damage, DOUBLED) ~= 0 then
        me.damage = bit32.bor(bit32.band(damage, FLAGS), bit32.band(damage, AMOUNT) * 2)
    end
    me.max_hp = MAX_HP
    local hp = MAX_HP - bit32.band(me.damage, AMOUNT)
    me.hp = int.u16(hp)
    s.last_hp = int.u32(hp)
    me.timer = LIFETIME
    if not me:create_collision() then
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        me:free()
        return
    end
    me:setup_collision(collision.neutral_object, collision.everything, HIT_MOD)
    local body = BODY_DAMAGE[s.level]
    if body == nil then
        error(string.format("LilBoiler level %d reads past dword_80D7748", s.level))
    end
    me.collision.self_damage = body
    me.collision.counter_byte = COUNTER
    me.collision:set_region(nil)
    me.collision:set_hit_effect(sparks.breaking)
    me.collision:present()
    battle.show_hp(me, HP_NUMBER)
    me.lifecycle = "update"
    tick(me)
end

boiler.kind = define.kind {
    id = "bn6:lilbolr/boiler",
    pool = "attack",
    state = {
        level = "u8",
        target_x = "u8",
        target_y = "u8",
        angle = "u8",
        hits = "u32",
        last_hp = "u32",
        erupting = "bool",
        steam = "object",
    },
    -- `sub_80D75FC`.
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

-- `sub_80D7A96`: a thrower for `throw.action` that throws a LilBoiler of
-- `level` at the panel three ahead of the user (the original keeps it as a
-- halfword from the user's panel). It registers it as one of its user's
-- side's field objects (`sub_80D7A78`: the "side" is the user's alliance
-- and flip halfword, which the spawner has just copied to the boiler; the
-- Atk+ bonus it popped into that register is overwritten first). A flipped
-- user's would land far past the registry
-- (`setFieldBattleObject_800F614` at BattleState+0xA8 + 12 x 0x100): no
-- player is flipped, and the port refuses it.
function boiler.thrower(level: number): Thrower
    return {
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local xy = int.u16(user.panel_x + user.panel_y * 0x100 + 3 * user.facing)
            local o = battle.spawn(boiler.kind, pos)
            if o then
                local s = o.state :: State
                s.level = level
                s.target_x = bit32.band(xy, 0xFF)
                s.target_y = bit32.rshift(xy, 8)
                o:set_damage_word(damage)
                o.alliance, o.flip = user.alliance, user.flip
                if user.flip ~= 0 then
                    error("LilBoiler thrown by a flipped user: setFieldBattleObject_800F614 registers it far past the field-object registry")
                end
                obstacle.register(o, user.alliance, 1)
            end
            return o
        end,
    }
end

return boiler
