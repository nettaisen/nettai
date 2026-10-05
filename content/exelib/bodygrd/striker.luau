--!strict
-- BodyGrd's striker, as EXE5 and EXE6 share it (EXE6's effect object #0x6E,
-- `sub_80E8268`; EXE5's #0x6E, the same code): each game makes its own kind
-- (`striker.make_kind`, a `BodyGuardStrikerLook`). An unseen thrower, one
-- of its side's field objects (one a side) with 1 HP. Outside dimming,
-- every 20 ticks it drops a shuriken (./shuriken) on the enemy's player
-- navi, ten in all, until the battle is over, the navi is down or another
-- field object replaces it. EXE6's BodyGrd counter leaves it; EXE5's BodyGrd
-- sends it out at once.

local shuriken = require("./shuriken")

local striker = {}

local EVERY, STRIKES = 0x14, 0xB
local HIT_MOD = 1

-- Gone: out of the field-object registry, freed next update.
local function leave(me: Object)
    -- (On the battle's end, the strikes' end or no target, the original
    -- first clears the word its ExtraVars+0 points at: nothing set it, so
    -- the store lands at address 0.)
    obstacle.unregister(me)
    me:set_lifecycle("destroy")
end

-- `sub_80E8326`: its target's panel: in a netbattle the other side's
-- player navi, while it has HP (`sub_80E8378`); offline, the other side's
-- combatant with the most HP, the later slot on a tie.
local function target(me: Object): (number?, number?)
    local other = bit32.bxor(me.alliance, 1)
    if battle.link() then
        local p = battle.player(other)
        if not p then
            error("BodyGrd's striker: sub_80103BC finds no player navi and sub_80E8378 reads its HP from address 0")
        end
        if p.hp == 0 then
            return nil, nil
        end
        return p.panel_x, p.panel_y
    end
    local best: Object? = nil
    for _, o in battle.alive_actors(other) do
        if best == nil or o.hp >= best.hp then
            best = o
        end
    end
    if not best then
        return nil, nil
    end
    return best.panel_x, best.panel_y
end

-- A striker's kind of a game's `look`.
function striker.make_kind(look: BodyGuardStrikerLook): Kind
    -- `sub_80E82D4`: every 20 ticks, a shuriken on the target.
    local function strike(me: Object)
        if battle.over() then
            leave(me)
            return
        end
        if me.hp == 0 then
            leave(me)
            return
        end
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        local left = me.timer2 - 1
        me.timer2 = left
        if left == 0 then
            leave(me)
            return
        end
        local x, y = target(me)
        if not x or not y then
            leave(me)
            return
        end
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        shuriken.spawn(look.shuriken, me, x, y, me.element, HIT_MOD, damage)
        battle.play_sound(look.throw_sound)
        me.timer = EVERY
    end

    -- `sub_80E829E`: nothing while dimmed; by phase.
    local function tick(me: Object)
        if battle.dimmed() then
            return
        end
        if me.phase == 0 then
            -- sub_80E82C0
            me.timer2, me.timer = STRIKES, EVERY
            me.phase, me.phase_init = 4, 0
        elseif me.phase ~= 4 then
            error(string.format("BodyGrd's striker: phase %#x reads past off_80E82B8", me.phase))
        end
        strike(me)
    end

    return define.kind {
        id = look.id,
        pool = "effect",
        -- `sub_80E8268`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- sub_80E8288
                me.identity = look.identity
                me.hp, me.max_hp = 1, 1
                me:set_lifecycle("update")
                tick(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }
end

-- `sub_80E8396`: a striker of `kind` with `element` and the damage word, of
-- `user`'s side; its side's one field object of class 1. (Its position is
-- the spawner's registers, which nothing reads.)
function striker.spawn(kind: Kind, user: Object, element: number, damage: number): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    o.element = element
    o.alliance = user.alliance
    o:set_damage_word(damage)
    obstacle.register(o, user.alliance, 1)
    return o
end

return striker
