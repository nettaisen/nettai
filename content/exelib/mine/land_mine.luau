--!strict
-- The mine the Mine chip lays, as BN5 and BN6 share it (BN6's attack
-- object #0x4C, `sub_80CDD44`; BN5's #0x4C): each game makes its own kind
-- (`land_mine.make_kind`: its identity, sprite, sounds, blast and
-- collision types). A field object that hops among the enemy's free panels
-- in a shuffled order for two seconds, then hides where it stopped, armed:
-- whatever touches it takes its damage, and it blows up when it hits or is
-- hit, when its HP runs out, when its panel stops being solid, or at the
-- end of the battle. It keeps its own action table (it doesn't go through
-- the obstacle framework's reactions).
--
-- Its spawner (`sub_80CDF60`) is exported: `land_mine.spawn(kind, side,
-- damage)`.

local panels = require("../panels")
local regions = require("../regions")

export type State = {
    -- The enemy panels it hops among, shuffled (the object's +0x60), how
    -- many (+0x74), and which it is on (+0x78).
    targets: { number },
    count: number,
    index: number,
    -- Ticks to the next hop (the original's third parameter).
    hop_ticks: number,
}

local land_mine = {}

-- A game's mine: its kind's key, its identity (a field object nothing
-- absorbs, swallows or leaves as junk, with no look), its sprite, its
-- hop's sound, its blast (BN6's effect 0x47) and the blast's sound, and its
-- collision types (what it is, and what it hits).
export type Look = {
    id: string,
    identity: Identity,
    sprite: SpriteAsset,
    hop_sound: Sound,
    blast: Effect,
    blast_sound: Sound,
    self_type: CollisionType,
    target: CollisionType,
}

-- It stays this long (`Timer`, which nothing counts down).
local LIFETIME = 0x1770
-- Hops: one every 2 ticks, 59 of them before it arms (`Timer2`).
local HOP_TICKS, HOPS = 2, 0x3C
-- The shuffle's swaps.
local SHUFFLE_SWAPS = 0x14
-- Where it hops (`byte_80CDF50`, by side): a free solid enemy panel
-- (side 0: flags 0x30, not 0x0F8C0080; side 1: 0x10, not 0x0F8C00A0).
local TARGETS: { [number]: PanelCondition } = {
    [0] = { require = 0x30, forbid = 0x0F8C_0080 },
    [1] = { require = 0x10, forbid = 0x0F8C_00A0 },
}
-- Its collision's hit modifier (it is a mine, and reaches what stands on
-- either side).
local HIT_MOD = 3

-- Its actions (`off_80CDE20`, by index).
local DESTROYED, ARMED = 2, 8

-- Hop to the next panel of the list (from the last back to the first).
local function hop(me: Object, look: Look)
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
    battle.play_sound(look.hop_sound)
end

-- `sub_80CDE68`: every 2 ticks, a hop; after 59, hidden and armed on its
-- panel.
local function hopping(me: Object, look: Look)
    local s = me.state :: State
    local n = s.hop_ticks - 1
    s.hop_ticks = int.u8(n)
    if n ~= 0 then
        return
    end
    s.hop_ticks = HOP_TICKS
    local t = me.timer2 - 1
    me.timer2 = t
    if t ~= 0 then
        hop(me, look)
        return
    end
    me.collision:set_region(regions.single)
    me.visible = false
    me.action, me.phase, me.phase_init = obstacle.action_byte(me, ARMED), 0, 0
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
    me.collision:set_region(nil)
    me.action, me.phase, me.phase_init = DESTROYED, 0, 0
end

-- `sub_80CDE44`: out of the field-object slots, hidden; it blows up at
-- its next update.
local function destroyed(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CDDFC`: hits (a push keeps the damage), its action
-- (`off_80CDE20`: its own actions and the framework's shared ones, which
-- nothing sets: it has no reactions), then its collision on the panel
-- again.
local function tick(me: Object, look: Look)
    me.sprite.white = false
    obstacle.take_hits(me, "keeps_damage")
    local a = obstacle.current(me)
    if a == 0 then
        hopping(me, look)
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
    elseif a == ARMED then
        armed(me)
    else
        error(string.format("the mine's action %d reads past its table", a))
    end
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
local function init(me: Object, look: Look)
    me.visible = true
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    -- (sub_8002EAC sets a drawing flag of the sprite's.)
    me.identity = look.identity
    me.timer = LIFETIME
    me.timer2 = HOPS
    local s = me.state :: State
    s.hop_ticks = HOP_TICKS
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
    s.index = 0
    if not me:create_collision() then
        fail(me)
        return
    end
    me:setup_collision(look.self_type, look.target, HIT_MOD)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me, look)
end

-- `sub_80CDEF8`: the blast, and gone.
local function blow_up(me: Object, look: Look)
    battle.play_sound(look.blast_sound)
    battle.effect(me.pos, look.blast)
    me.collision:free()
    me:free()
end

-- A game's mine kind.
function land_mine.make_kind(look: Look): Kind
    -- `sub_80CDD44`.
    local function update(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me, look)
        elseif l == "update" then
            tick(me, look)
        else
            blow_up(me, look)
        end
    end

    return define.kind {
        id = look.id,
        pool = "attack",
        state = { targets = "u8[18]", count = "u32", index = "u32", hop_ticks = "u8" },
        update = update,
    }
end

-- `sub_80CDF60`: a mine (of `kind`, a game's) for `side` with the damage
-- word `damage`, in the side's field-object slots (class 0: two a side).
-- (Its position is the spawner's registers, which its init replaces.)
function land_mine.spawn(kind: Kind, side: number, damage: number): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if not o then
        return nil
    end
    o.alliance = side
    o:set_damage_word(damage)
    o.run_while_dimmed = true
    obstacle.register(o, side, 0)
    return o
end

return land_mine
