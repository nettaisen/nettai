--!strict
-- The mine the Mine chip lays (attack object #0x4C, `sub_80CDD44`): a
-- field object that hops among the enemy's free panels in a shuffled
-- order for two seconds, then hides where it stopped, armed: whatever
-- touches it takes its damage, and it blows up (a puff, effect #0 look
-- 0x47) when it hits or is hit, when its HP runs out, when its panel
-- stops being solid, or at the end of the battle. It keeps its own action
-- table (it doesn't go through the obstacle framework's reactions).
--
-- Its spawner (`sub_80CDF60`) is exported: `land_mine.spawn(side, damage)`.

local panels = require("../../lib/panels")

export type State = {
    -- The enemy panels it hops among, shuffled (the object's +0x60), how
    -- many (+0x74), and which it is on (+0x78).
    targets: { number },
    count: number,
    index: number,
}

local land_mine = {
    state = { targets = "u8[18]", count = "u32", index = "u32" },
}

local SPRITE = "0C-22"
local NAME = 0xDA
-- It stays this long (`Timer`, which nothing counts down).
local LIFETIME = 0x1770
-- Hops: one every 2 ticks (Param3), 59 of them before it arms (`Timer2`).
local HOP_TICKS, HOPS = 2, 0x3C
local HOP_SOUND = 0x113
-- The shuffle's swaps.
local SHUFFLE_SWAPS = 0x14
-- Where it hops (`byte_80CDF50`, by side): a free solid enemy panel
-- (side 0: flags 0x30, not 0x0F8C0080; side 1: 0x10, not 0x0F8C00A0).
local TARGETS: { [number]: PanelCondition } = {
    [0] = { require = 0x30, forbid = 0x0F8C_0080 },
    [1] = { require = 0x10, forbid = 0x0F8C_00A0 },
}
-- Its collision: types 0x33 and 0x2A, hit modifier 3.
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 0x33, 0x2A, 3
-- The blast: effect #0 look 0x47 and its sound.
local BLAST, BLAST_SOUND = 0x47, 0x70

-- Its actions (`off_80CDE20`, by index).
local DESTROYED, ARMED = 2, 8

-- `sub_80CDF60`: a mine for `side` with the damage word `damage`, in the
-- side's field-object slots (class 0: two a side). (Its position and
-- parameters are the spawner's registers, which its init replaces or
-- nothing reads.)
function land_mine.spawn(side: number, damage: number): Object?
    local o = battle.spawn_kind("land-mine", Vec3.zero)
    if not o then
        return nil
    end
    o.alliance = side
    o.damage = bit32.band(damage, 0xFFFF)
    o.stamina = bit32.rshift(damage, 16)
    o.run_while_dimmed = true
    obstacle.register(o, side, 0)
    return o
end

-- Hop to the next panel of the list (from the last back to the first).
local function hop(me: Object)
    local s = me.state :: State
    local i = s.index + 1
    if not (s.count > i) then
        i = 0
    end
    s.index = i
    local p = s.targets[i + 1]
    me.panel_x, me.panel_y = panels.x(p), panels.y(p)
    me:update_collision_panels()
    me:set_coordinates_from_panel()
    battle.play_sound(HOP_SOUND)
end

-- `sub_80CDE68`: every 2 ticks, a hop; after 59, hidden and armed on its
-- panel.
local function hopping(me: Object)
    local n = me:param(3) - 1
    me:set_param(3, int.u8(n))
    if n ~= 0 then
        return
    end
    me:set_param(3, HOP_TICKS)
    local t = me.timer2 - 1
    me.timer2 = t
    if t ~= 0 then
        hop(me)
        return
    end
    me.collision.region = 1
    me.visible = false
    me.action, me.phase, me.phase_init = ARMED, 0, 0
end

-- `sub_80CDEC8`: armed, until it touched something (or something it),
-- its HP ran out, its panel isn't solid or the battle is over.
local function armed(me: Object)
    if
        me.hp ~= 0
        and me.collision.hit_flags == 0
        and field.solid(me.panel_x, me.panel_y)
        and not battle.over()
    then
        return
    end
    me.collision.region = 0
    me.action, me.phase, me.phase_init = DESTROYED, 0, 0
end

-- `sub_80CDE44`: out of the field-object slots, hidden; it blows up at
-- its next update.
local function destroyed(me: Object)
    me.collision.region = 0
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- The mine's action table (`off_80CDE20`): its own actions and the
-- framework's shared ones (which nothing sets: it has no reactions).
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = hopping,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = armed,
}

-- `sub_80CDDFC`: hits (a push keeps the damage), its action, then its
-- collision on the panel again.
local function tick(me: Object)
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    local a = me.action
    local action = ACTIONS[a]
    if not action then
        error(string.format("the mine's action %d reads past its table", a))
    end
    action(me)
    me.collision:present()
    me:update_sprite()
end

-- `sub_80CDF18`: the enemy's free panels, shuffled; the first one (nil:
-- none).
local function first_panel(me: Object): (number?, number?)
    local cond = TARGETS[me.alliance]
    local list = panels.filtered_except(me, cond.require, cond.forbid)
    local n = #list
    if n == 0 then
        return nil, nil
    end
    local s = me.state :: State
    s.count = n
    panels.shuffle(list, n, SHUFFLE_SWAPS)
    for i, p in list do
        s.targets[i] = p
    end
    return panels.x(list[1]), panels.y(list[1])
end

-- It can't be laid: out of the slots and gone.
local function fail(me: Object)
    obstacle.unregister(me)
    me.visible = false
    me:free()
end

-- `sub_80CDD64`: on the first of the shuffled panels, hopping; with no
-- enemy panel free, gone.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    -- (sub_8002EAC sets a drawing flag of the sprite's.)
    me.name_id = NAME
    me.timer = LIFETIME
    me.timer2 = HOPS
    me:set_param(3, HOP_TICKS)
    me.hp, me.max_hp = 1, 1
    local x, y = first_panel(me)
    if not x or not y then
        fail(me)
        return
    end
    -- (The game also writes the panel into its collision data, which it
    -- doesn't have yet: through a null pointer.)
    me.panel_x, me.panel_y = x, y
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    local s = me.state :: State
    s.index = 0
    if not me:create_collision() then
        fail(me)
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    me.collision.region = 0
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80CDEF8`: the blast, and gone.
local function blow_up(me: Object)
    battle.play_sound(BLAST_SOUND)
    battle.effect(me.pos, BLAST)
    me.collision:free()
    me:free()
end

-- `sub_80CDD44`.
function land_mine.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        blow_up(me)
    end
end

return land_mine
