--!strict
-- SearchMan's shot, as EXE5 and EXE4 share it (EXE5's attack object #0x57, 0x080D0D78; EXE4's #0x57, 0x080D87B0; EXE6
-- keeps its code, `sub_80CF5C8`, which no EXE6 chip spawns): a one-tick hit on a panel, at the game's height; a miss
-- shows a spark near where it struck, flipped up or down at random. The powered last shot is another hit and shows a
-- shot's impact. Each game makes its kind (`shot.make`): its height (EXE5's 16 pixels up, EXE4's on the ground), its
-- miss's spark (an effect row), its collision types and sparks.

local shot = {}

export type State = {
    -- The powered shot (its Param1 29, not 25).
    powered: boolean,
}

export type Spec = {
    id: string,
    -- Its height (16.16).
    z: number,
    -- A miss's spark.
    miss: Effect,
    -- What it is (plain and powered: the rows 25 and 29 its Param1 names), what it hits, and its hit's spark (plain
    -- and powered: sparks 0 and 9).
    plain_type: CollisionType,
    powered_type: CollisionType,
    target: CollisionType,
    plain_spark: Spark,
    powered_spark: Spark,
}

export type Shot = {
    kind: Kind,
    spawn: (searchman: Object, x: number, y: number, powered: boolean) -> Object?,
}

function shot.make(spec: Spec): Shot
    -- 0x080D0D98 (EXE4's 0x080D87D0): on its panel at its height, its hit there (gone off the field).
    local function init(me: Object)
        if not field.valid(me.panel_x, me.panel_y) then
            me:free()
            return
        end
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, spec.z)
        if not me:create_collision() then
            me:free()
            return
        end
        local s = me.state :: State
        me:setup_collision(if s.powered then spec.powered_type else spec.plain_type, spec.target, 3)
        me.collision:set_hit_effect(if s.powered then spec.powered_spark else spec.plain_spark)
        me.collision:present()
        me:set_lifecycle("update")
    end

    -- 0x080D0DDE (EXE4's 0x080D8814): its hits resolved; a miss's spark; gone.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if c.hit_flags == 0 then
            local flip = bit32.band(battle.rng_positive(), 2)
            battle.effect(battle.jitter(0xF, me.pos), spec.miss, flip, 0, 0)
        end
        c:set_region(nil)
        c:free()
        me:free()
    end

    local kind = new.kind {
        id = spec.id,
        pool = "attack",
        state = { powered = "bool" },
        -- 0x080D0D78.
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

    -- 0x080D0E1E (EXE4's 0x080D8854): a shot of `searchman` on (x, y) with his damage word.
    local function spawn(searchman: Object, x: number, y: number, powered: boolean): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if o then
            o.panel_x, o.panel_y = x, y
            o:set_damage_word(bit32.bor(searchman.damage, bit32.lshift(searchman.stamina, 16)))
            o.alliance, o.flip = searchman.alliance, searchman.flip
            o.run_while_dimmed = true
            ;(o.state :: State).powered = powered
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return shot
