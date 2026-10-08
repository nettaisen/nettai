--!strict
-- The north wind (attack object #0x5A: EXE6's `sub_80CFC08`, code no EXE6 chip uses; EXE5's 0x080D122C, NrthWind's;
-- EXE4's 0x080D8BC8, its NrthWind's and WindSoul's change's) and its streaks (effect object #0x3A, EXE6's
-- `sub_80E4848`): 100 ticks of streaks, one every 8 somewhere along its side's back edge; then its region over the
-- whole field for a tick (row 0x1B: what wind hits remove barriers by), its holder told it is done.
--
-- Each game makes its own (`wind.make`): its ids, its sound as it starts and its streaks' sprite, its collision types
-- (row 0x1B, nothing), and whether its spawn gives it its owner's flip with its side (EXE6's and EXE5's,
-- `sub_80CFCD6`: the halfword; EXE4's 0x080D8C94 copies the side alone, as its streaks' spawn does).

local regions = require("../regions")

local wind = {}

-- A game's north wind: the wind's kind, its streak's, and `blow`: a north wind from `owner` (running while dimmed),
-- telling `holder` it is done (its state's `busy` cleared), if any.
export type NorthWind = {
    kind: Kind,
    streak: Kind,
    blow: (owner: Object, holder: Object?) -> Object?,
}

-- Its ticks, and a streak's every.
local WIND_TICKS, STREAK_EVERY = 0x64, 8
-- A streak: blown forward 16 pixels a tick, thrown up and falling, for 30 ticks.
local STREAK_SPEED, STREAK_RISE, STREAK_GRAVITY, STREAK_TICKS = 0x10_0000, int.i32(0xFFF98000), 0x3C00, 0x1E

type WindState = { holder: Object? }

function wind.make(spec: {
    id: string,
    sound: Sound,
    streak_sprite: SpriteAsset,
    is: CollisionType,
    hits: CollisionType,
    copies_flip: boolean,
}): NorthWind
    -- EXE6's `sub_80E48C0`.
    local function streak_tick(me: Object)
        me:update_sprite()
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(int.i32(p.x + v.x), p.y, int.i32(p.z + v.z))
        me.vel = Vec3.new(v.x, v.y, int.i32(v.z + STREAK_GRAVITY))
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            me:set_lifecycle("destroy")
        end
    end

    local streak_kind = new.kind {
        id = spec.id .. "/streak",
        pool = "effect",
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- EXE6's `sub_80E4868`: its animation 0 in palette 0 (its first parameter).
                me.visible = true
                local sprite = me.sprite
                sprite:load(spec.streak_sprite)
                -- (`sprite_noShadow`: the first part is drawn with the sprite.)
                sprite.shadow = "with_sprite"
                me.anim, me.anim_loaded = 0, 0
                sprite:set_animation(0)
                sprite.palette = 0
                sprite:set_flip(bit32.bxor(me.alliance, me.flip))
                me.vel = Vec3.new(me.facing * STREAK_SPEED, me.vel.y, STREAK_RISE)
                me.timer = STREAK_TICKS
                me.lifecycle = "update"
                streak_tick(me)
            elseif l == "update" then
                streak_tick(me)
            else
                me:free()
            end
        end,
    }

    -- EXE6's `sub_80CFCA8` and `sub_80E48F8`: a streak at random along the back edge of the wind's side's view (one
    -- GetRNG2 draw: x from its low 5 bits, -0x98 pixels and on, the other edge facing left; y from its top 7, -0x40
    -- and on), 40 pixels up.
    local function streak(me: Object)
        local r = int.u32(battle.rng())
        local x = bit32.band(r, 0x1F) - 0x98
        if bit32.bxor(me.alliance, me.flip) ~= 0 then
            x += 0x110
        end
        local y = bit32.rshift(r, 25) - 0x40
        local o = battle.spawn(streak_kind, Vec3.new(int.i32(x * 0x10000), int.i32(y * 0x10000), 0x28_0000))
        if o then
            o.alliance = me.alliance
            if spec.copies_flip then
                o.flip = me.flip
            end
        end
    end

    local function wind_tick(me: Object)
        local c = me.collision
        c:remove()
        local t2 = me.timer2 - 1
        me.timer2 = t2
        if t2 == 0 then
            streak(me)
            me.timer2 = STREAK_EVERY
        end
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            c:set_region(regions.field)
            local holder = (me.state :: WindState).holder
            if holder then
                (holder.state :: { busy: boolean }).busy = false
            end
            me:set_lifecycle("destroy")
        end
        c:present()
    end

    local wind_kind = new.kind {
        id = spec.id .. "/wind",
        pool = "attack",
        state = { holder = "object" },
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- EXE6's `sub_80CFC28`.
                me.timer = WIND_TICKS
                me.timer2 = STREAK_EVERY
                if not me:create_collision() then
                    me:free()
                    return
                end
                me:setup_collision(spec.is, spec.hits, 0)
                me.collision:set_region(nil)
                battle.play_sound(spec.sound)
                me.lifecycle = "update"
                wind_tick(me)
            elseif l == "update" then
                wind_tick(me)
            else
                -- EXE6's `sub_80CFC96`.
                me.collision:remove()
                me:free()
            end
        end,
    }

    -- EXE6's `sub_80CFCD6`: of `owner`'s side (and flip), running while dimmed (its header flag 0x10).
    local function blow(owner: Object, holder: Object?): Object?
        local o = battle.spawn(wind_kind)
        if o then
            (o.state :: WindState).holder = holder
            o.alliance = owner.alliance
            if spec.copies_flip then
                o.flip = owner.flip
            end
            o.run_while_dimmed = true
        end
        return o
    end

    return { kind = wind_kind, streak = streak_kind, blow = blow }
end

return wind
