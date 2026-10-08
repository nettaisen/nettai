--!strict
-- EXE4's lingering hit (attack object #0x7C, 0x080DCC5C: EXE4's own, EXE6's #0x7C is another object): a hit region set
-- off its owner's panel by a step, which follows its owner as it moves and lasts its ticks; a hit on a body ends it.
-- While its owner is paralyzed, dragged or sliding (its collision's flags 0x101800) its ticks hold. A holder may keep
-- it in a slot of its state's `hits` (the original's word its spawner was given), which it fills as it starts and
-- empties as it goes.
--
-- Each user makes its kind (`lingering_hit.make`): BurnMan's pillars and flame (chips/burnman), AquaMan's.

local lingering_hit = {}

export type Spec = {
    -- Its region and spark, what it hits and what it is (the spawner's parameters 1 to 4), its hit modifier, status
    -- and NaviCust bug (its r7's bytes), its damage word, its height, its ticks and element.
    region: Region,
    hit_effect: Spark?,
    target: CollisionType,
    self_type: CollisionType,
    hit_mod: number,
    status: Status?,
    bug: number?,
    bug_arg: number?,
    damage: number,
    z: number,
    ticks: number,
    element: number,
    -- The holder that keeps it, and the slot of its `hits` (1 to 3).
    holder: Object?,
    slot: number?,
}

export type LingeringHit = {
    kind: Kind,
    spawn: (owner: Object, dx: number, dy: number, spec: Spec) -> Object?,
}

type State = {
    dx: number,
    dy: number,
    region: Region?,
    hit_effect: Spark?,
    target: CollisionType?,
    self_type: CollisionType?,
    hit_mod: number,
    status: Status?,
    bug: number,
    bug_arg: number,
    holder: Object?,
    slot: number,
}

-- Its owner holds it while paralyzed, dragged or sliding.
local function held(owner: Object): boolean
    if not owner.has_collision then
        return false
    end
    return owner:status("paralyzed") or owner:status("drag") or owner:status("sliding")
end

function lingering_hit.make(id: string): LingeringHit
    -- Its holder's slot, filled with it or emptied (while its holder is one of its holders' kinds still).
    local function keep(me: Object, it: Object?)
        local s = me.state :: State
        local h = s.holder
        if h and s.slot ~= 0 and h.active then
            ((h.state :: any).hits :: { Object? })[s.slot] = it
        end
    end

    -- 0x080DCCE8: its hits resolve and spark; a hit on a body ends it; else its ticks (held with its owner), and on
    -- its owner's panel and step.
    local function tick(me: Object)
        local s = me.state :: State
        local c = me.collision
        c:remove()
        me:hit_spark()
        if c.hit_flags == 0 then
            local owner = me.related1 :: Object
            local ended = false
            if not held(owner) then
                local left = me.timer - 1
                me.timer = int.u16(left)
                ended = left <= 0
            end
            if not ended then
                me.panel_x, me.panel_y = owner.panel_x + s.dx, owner.panel_y + s.dy
                me:set_coordinates_from_panel()
                me:update_collision_panels()
                c:present()
                return
            end
        end
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- 0x080DCC8E: on its panel's center; its collision (freed without one), its region, spark, status and bug; kept
    -- by its holder; then its first tick.
    local function init(me: Object)
        local s = me.state :: State
        me:set_coordinates_from_panel()
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(s.self_type :: CollisionType, s.target :: CollisionType, s.hit_mod)
        local c = me.collision
        c:set_region(s.region)
        c:set_hit_effect(s.hit_effect)
        c.status_base = s.status
        if s.bug ~= 0 then
            c.bugs = s.bug + s.bug_arg * 0x100
        end
        c:present()
        keep(me, me)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = id,
        pool = "attack",
        state = {
            dx = "i8",
            dy = "i8",
            region = "region",
            hit_effect = "spark",
            target = "collision",
            self_type = "collision",
            hit_mod = "u8",
            status = "status",
            bug = "u8",
            bug_arg = "u8",
            holder = "object",
            slot = "u8",
        },
        -- 0x080DCC5C.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- 0x080DCC7C: out of its holder's slot; its collision, then itself, freed.
                keep(me, nil)
                me.collision:free()
                me:free()
            end
        end,
    }

    -- 0x080DCD3A, then the callers' part (0x080D4002): a hit `dx`, `dy` panels off `owner`'s, of its side, as `spec`
    -- says, running while dimmed when `owner` does. (Its position is the spawner's registers, which its init
    -- overwrites but the height.)
    local function spawn(owner: Object, dx: number, dy: number, spec: Spec): Object?
        local o = battle.spawn(kind, Vec3.new(dy, 0, spec.z))
        if not o then
            return nil
        end
        local s = o.state :: State
        s.dx, s.dy = dx, dy
        o.panel_x, o.panel_y = owner.panel_x + dx, owner.panel_y + dy
        o.alliance = owner.alliance
        o:set_damage_word(spec.damage)
        s.region, s.hit_effect, s.target, s.self_type = spec.region, spec.hit_effect, spec.target, spec.self_type
        s.hit_mod, s.status, s.bug, s.bug_arg = spec.hit_mod, spec.status, spec.bug or 0, spec.bug_arg or 0
        s.holder, s.slot = spec.holder, spec.slot or 0
        o.related1 = owner
        o.timer = spec.ticks
        o.element = spec.element
        o.run_while_dimmed = owner.run_while_dimmed
        return o
    end

    return { kind = kind, spawn = spawn }
end

return lingering_hit
