--!strict
-- Geyser's bomb and water, as EXE5 and EXE4 share them (EXE5's attack objects #0x42, 0x080CD992, and #0x43, 0x080CDC2C,
-- thrown by action 0x12's subtype 5, 0x080CDC08: EXE6's BlkBomb code, `sub_80CD4EC`, reworked; EXE4's #0x42,
-- 0x080D58EE, and #0x43, 0x080D5B84, thrown by action 0x09's variant 5, 0x080D5B62): an aqua bomb lobbed at the panel
-- three ahead. Landing on a solid panel it only splashes it for 10 damage; landing in a hole it erupts: a geyser that
-- hits its panel while it stands and, two ticks in, sends water onto the eight panels around it, one every three ticks
-- in a shuffled order. Off the field it just goes. Each game makes its own (`geyser.make`) with its look: its kinds,
-- sprites, splash, sounds and collision types, and whether the splash takes the throw's Atk+ bonus (EXE4's).

local panels = require("../panels")
local regions = require("../regions")

local geyser = {}

type State = {
    -- The panel it aims at (Param3 and Param4).
    target_x: number,
    target_y: number,
    -- The throw's Atk+ bonus (+0x64, the thrower's register: EXE5 stores it and never reads it), a look's whose
    -- splash takes it.
    bonus: number?,
}

type WaterState = {
    -- The eight panels' order (ExtraVars): indices into `AROUND`, each half shuffled.
    order: { number },
    -- How many it has hit (+0x22).
    count: number,
}

local SPEED, GRAVITY = 0x2E666, -0x2800
local HIT_MOD = 3
-- The splash's damage on solid ground (its damage word's hit parameter, damage 10).
local SPLASH_DAMAGE = 10
-- The geyser's water: two ticks in; then a panel every three ticks.
local WATER_AT, WATER_TICKS = 2, 3
-- The geyser stands this long between its rise and its fall.
local STAND_TICKS = 15
-- The panels around it (EXE5's 0x080CDCC8, EXE4's 0x080D5C20): dx toward the facing side, dy.
local AROUND = {
    [0] = { -1, 0 },
    [1] = { 0, -1 },
    [2] = { 1, 0 },
    [3] = { 0, 1 },
    [4] = { -1, -1 },
    [5] = { 1, -1 },
    [6] = { -1, 1 },
    [7] = { 1, 1 },
}
-- Its element: aqua.
local AQUA = 2
-- Its target: three panels ahead.
local REACH = 3

-- A game's Geyser, aimed over its sine table.
function geyser.make(look: GeyserLook, aim: Trajectory): GeyserBomb
    -- An aqua hit on (x, y) at height `z` of damage word `word`: an attack reaching navis, the aqua spark (0x04050201).
    local function hit(me: Object, x: number, y: number, z: number, word: number)
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = z,
            region = regions.single,
            hit_effect = look.spark,
            target = look.hits,
            self_type = look.attack,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = HIT_MOD,
        })
    end

    -- EXE5's 0x080CDC2C: the order on its first tick; a panel every three ticks; gone after the eighth.
    local water_kind = new.kind {
        id = look.water_id,
        pool = "attack",
        state = { order = "u8[8]", count = "u16" },
        update = function(me: Object)
            local s = me.state :: WaterState
            if me.phase_init == 0 then
                for i = 1, 8 do
                    s.order[i] = i - 1
                end
                -- (Each half, four swaps: `sub_8000C72` on ExtraVars and ExtraVars+4.)
                local first, second = { s.order[1], s.order[2], s.order[3], s.order[4] }, { s.order[5], s.order[6], s.order[7], s.order[8] }
                panels.shuffle(first, 4, 4)
                panels.shuffle(second, 4, 4)
                for i = 1, 4 do
                    s.order[i], s.order[i + 4] = first[i], second[i]
                end
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t > 0 then
                return
            end
            me.timer = WATER_TICKS
            local d = AROUND[s.order[s.count + 1]]
            local x, y = me.panel_x + me.facing * d[1], me.panel_y + d[2]
            hit(me, x, y, me.pos.z, int.u32(me.damage + me.stamina * 0x10000))
            local cx, cy = field.center(x, y)
            battle.effect(Vec3.new(cx, cy, 0), look.splash)
            s.count += 1
            if s.count >= 8 then
                me:free()
            end
        end,
    }

    -- `sub_80CC918`'s counterpart (EXE5's 0x080CDCD8): the water on (x, y), of the geyser's element and damage. (Its
    -- position is the spawner's registers: the panel and the element.)
    local function water(me: Object, x: number, y: number)
        local o = battle.spawn(water_kind, Vec3.new(x, y, me.element))
        if o then
            o.panel_x, o.panel_y = x, y
            o:set_damage_word(int.u32(me.damage + me.stamina * 0x10000))
            o.element = me.element
            o.alliance, o.flip = me.alliance, me.flip
        end
    end

    -- Gone: no region, and freed at the next update.
    local function leave(me: Object)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- EXE5's 0x080CDAAC: fly; landed, a splash or a geyser.
    local function fly(me: Object)
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t >= 0 then
            local v = me.vel
            local vz = int.i32(v.z + GRAVITY)
            me.vel = Vec3.new(v.x, v.y, vz)
            local p = me.pos
            local z = int.i32(p.z + vz)
            me.pos = Vec3.new(p.x + v.x, p.y + v.y, z)
            if z >= 0 then
                return
            end
        end
        me:set_panel_from_coordinates()
        local x, y = me.panel_x, me.panel_y
        me.collision.panel_x, me.collision.panel_y = x, y
        if not field.valid(x, y) then
            leave(me)
            return
        end
        if field.solid(x, y) then
            -- EXE5's 0x080CDB36: a splash of 10 damage (EXE4's 0x080D5A92: and the throw's Atk+ bonus).
            local word = bit32.bor(bit32.band(int.u32(me.damage + me.stamina * 0x10000), 0xFFFF0000), SPLASH_DAMAGE)
            if look.splash_bonus then
                word = int.u32(word + ((me.state :: State).bonus :: number))
            end
            hit(me, x, y, 0, word)
            battle.effect(me.pos, look.splash)
            battle.play_sound(look.splash_sound)
            leave(me)
            return
        end
        -- In a hole: the geyser rises.
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me.collision:set_region(regions.single)
        local sprite = me.sprite
        sprite:load(look.geyser)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0xFF
        battle.play_sound(look.erupt)
        me.timer = 0
        me.action = 4
        me.phase, me.phase_init = 0, 0
    end

    -- EXE5's 0x080CDB7C: the rise; the water two ticks in.
    local function rise(me: Object)
        local t = me.timer + 1
        me.timer = t
        if t == WATER_AT then
            water(me, me.panel_x, me.panel_y)
        end
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim = 1
            me.timer = STAND_TICKS
            me.action = 8
            me.phase, me.phase_init = 0, 0
        end
    end

    -- EXE5's 0x080CDBB4: stand; then the fall.
    local function stand(me: Object)
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t == 0 then
            battle.play_sound(look.splash_sound)
            me.anim = 2
            me.action = 12
            me.phase, me.phase_init = 0, 0
        end
    end

    -- EXE5's 0x080CDA5C: the battle over, gone; a hit takes its region away; then its action.
    local function tick(me: Object)
        me.collision:remove()
        if battle.over() then
            me.collision:set_region(nil)
            me.visible = false
            me:set_lifecycle("destroy")
            return
        end
        if me.collision.hit_flags ~= 0 then
            me.collision:set_region(nil)
        end
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            rise(me)
        elseif a == 8 then
            stand(me)
        elseif a == 12 then
            -- EXE5's 0x080CDBD4: gone once its fall ends.
            if bit32.btest(me.sprite.frame_flags, 0x80) then
                leave(me)
            end
        else
            error(string.format("Geyser's action %#x reads past its table", a))
        end
        me.collision:present()
    end

    -- EXE5's 0x080CD9B4: its sprite; aimed at the target panel at its speed; its collision, no region yet.
    local function init(me: Object)
        local s = me.state :: State
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.bomb)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = 1, 0
        sprite:set_animation(1)
        sprite:step()
        local tx, ty = field.center(s.target_x, s.target_y)
        local p = me.pos
        local angle = aim.angle(int.i32(tx - p.x), int.i32(ty - p.y))
        local vx, vy = aim.toward(angle, SPEED)
        me.vel = Vec3.new(vx, vy, me.vel.z)
        local rise_z, ticks = aim.at_speed(me.pos, vy, tx, ty, 0, SPEED, GRAVITY)
        me.vel = Vec3.new(vx, vy, rise_z)
        me.timer = ticks
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.attack, look.hits, HIT_MOD)
        me.collision:set_region(nil)
        me.collision:present()
        me:set_lifecycle("update")
    end

    local schema: StateSchema = { target_x = "u8", target_y = "u8" }
    if look.splash_bonus then
        schema.bonus = "u16"
    end
    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = schema,
        -- EXE5's 0x080CD992.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
                if not me.active then
                    return
                end
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }

    -- EXE5's 0x080CDC08: a thrower of Geyser bombs at the panel three ahead of the user, aqua (with the throw's Atk+
    -- bonus, the attack's +6, when the look's splash takes it).
    local function thrower(): Thrower
        return {
            throw = function(user: Object, pos: Vec3, damage: number): Object?
                local o = battle.spawn(kind, pos)
                if o then
                    o:set_damage_word(damage)
                    o.element = AQUA
                    o.alliance = user.alliance
                    local s = o.state :: State
                    s.target_x, s.target_y = user.panel_x + REACH * user.facing, user.panel_y
                    if look.splash_bonus then
                        s.bonus = user.extra
                    end
                end
                return o
            end,
        }
    end

    return { kind = kind, water = water_kind, thrower = thrower }
end

return geyser
