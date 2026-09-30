--!strict
-- ElemTrap's trap (attack object #0x4D, `sub_80CDF84`): an invisible
-- collision over the whole field, even while the battle is dimmed, that
-- waits for its side's opponent to take elemental damage. The first hit
-- of fire, aqua, elec or wood springs it: once the battle isn't dimmed it
-- marks the enemy navi's panels with sparkles and hands over to a
-- counterattack of that element (objects/elem-trap-strike) in a dimming of
-- its own; the trap is gone. It also goes when its side's defensive chip
-- changes (the linked record: Param2), at the end of the battle, and if it
-- is hit in the tick it appears. See docs/engine/chips.md §3.6.9.
--
-- The trap chips' controller (objects/trap-chip) spawns it for Param1 0:
-- `elem_trap.spawn(controller, user)` (`sub_80E358C`).

local panels = require("../../lib/panels")
local elem_trap_strike = require("../elem-trap-strike/elem_trap_strike")

export type State = {
    -- The chip and the Atk+ / cross bonus (object +0x30, +0x32), passed to
    -- the counterattack.
    chip: number,
    bonus: number,
    -- Hits spring it (not in the tick it appears).
    armed: boolean,
}

local elem_trap = {
    state = { chip = "u16", bonus = "u16", armed = "bool" },
}

-- Its collision (`object_setupCollisionData` self 0, target 0x18, hit
-- modifier 0) covers the whole field: region 0x80.
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 0, 0x18, 0
local WHOLE_FIELD = 0x80

-- What springs it: damage in these elements (fire to wood; the first one
-- that has some).
local FIRST_ELEMENT, LAST_ELEMENT = 1, 4

-- Its actions (`off_80CE02C`, byte offsets).
local WATCH, SPRING = 0, 4

-- `byte_80CE0DC`: the panels its sparkles mark, by its side: the enemy
-- navi's (flags 0x00200000 for side 0's trap, 0x00400000 for side 1's).
local MARKED: { [number]: PanelCondition } = {
    [0] = { require = 0x0020_0000, forbid = 0 },
    [1] = { require = 0x0040_0000, forbid = 0 },
}
-- `sub_800ABC6`: the sparkle (effect #0 look 0x46, 16 pixels down and 32
-- up from the panel's center) and its sound.
local SPARKLE, SPARKLE_SOUND = 0x46, 0xA5
local SPARKLE_DY, SPARKLE_Z = 0x10_0000, 0x20_0000

-- `sub_80CE0EC`: the trap for `user`, on `controller`'s side, with the
-- controller's parameters, damage word and chip. (Its position is the
-- spawner's registers, which nothing reads.)
function elem_trap.spawn(controller: Object, user: Object, chip: number, bonus: number): Object?
    local params = { controller:param(1), controller:param(2), controller:param(3), controller:param(4) }
    local o = battle.spawn_kind("elem-trap", Vec3.new(0, controller.element, 0), params)
    if not o then
        return nil
    end
    o.related1 = user
    o.element = 0
    o.damage, o.stamina = controller.damage, controller.stamina
    o.alliance = controller.alliance
    local s = o.state :: State
    s.chip, s.bonus = chip, bonus
    o.run_while_dimmed = true
    return o
end

-- `sub_80CE0A4`: sparkles on the enemy navi's panels, first to last.
local function sparkle(me: Object)
    local marked = MARKED[me.alliance]
    local list = panels.filtered(marked.require, marked.forbid)
    for i = #list, 1, -1 do
        local p = list[i]
        local x, y = field.center(panels.x(p), panels.y(p))
        battle.effect(Vec3.new(x, y + SPARKLE_DY, SPARKLE_Z), SPARKLE, battle.local_side())
        battle.play_sound(SPARKLE_SOUND)
    end
end

-- `sub_80CE05E`: sprung. Hit as it appeared, it just goes; else, once the
-- battle isn't dimmed, the sparkles and the counterattack (its user's
-- side's dimming, which the other side can't cut in on), and it goes.
local function spring(me: Object)
    local s = me.state :: State
    if s.armed then
        if battle.dimmed() then
            return
        end
        sparkle(me)
        local user = me.related1
        if not user then
            error("ElemTrap's trap has no user (sub_80E360E reads through a null pointer)")
        end
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local strike = elem_trap_strike.spawn(user, me:param(1), damage, s.chip, s.bonus)
        dimming.start(user.alliance, true, strike, user)
    end
    battle.clear_linked(me.alliance)
    me:set_lifecycle("destroy")
end

-- `sub_80CE034`: the first of fire, aqua, elec and wood with damage this
-- tick springs it (Param1: the element).
local function watch(me: Object)
    for e = FIRST_ELEMENT, LAST_ELEMENT do
        if me.collision:element_damage(e) ~= 0 then
            me:set_param(1, e)
            me.collision.region = 0
            me.action, me.phase, me.phase_init = SPRING, 0, 0
            spring(me)
            return
        end
    end
end

-- The collision's panels and the object go (`object_clearCollisionRegion`,
-- `object_freeCollisionData`, `object_freeMemory`).
local function vanish(me: Object)
    me.collision.region = 0
    me.collision:free()
    me:free()
end

-- `sub_80CDFEE`: resolve this tick's hits; unless its record went or the
-- battle is over, watch or spring, then register again.
local function tick(me: Object)
    me.collision:remove()
    if me:param(2) ~= 0 then
        vanish(me)
        return
    end
    if battle.over() then
        battle.clear_linked(me.alliance)
        vanish(me)
        return
    end
    local a = me.action
    if a == WATCH then
        watch(me)
    elseif a == SPRING then
        spring(me)
    else
        error(string.format("ElemTrap's trap action %#x reads past its table", a))
    end
    me.collision:present()
end

-- `sub_80CDFA4`: its collision over the whole field, hit while dimmed; one
-- update unarmed, then armed.
local function init(me: Object)
    me.panel_x, me.panel_y = 1, 1
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    me.collision.region = WHOLE_FIELD
    me:set_status("hit_while_dimmed", true)
    me.collision:present()
    local s = me.state :: State
    s.armed = false
    me:set_lifecycle("update")
    tick(me)
    s.armed = true
end

-- `sub_80CDF84`.
function elem_trap.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return elem_trap
