--!strict
-- EXE5's LifeSync (the dimming handler's subtype 39): its dimming controller (effect object #0x5C, 0x080E710C: EXE6's
-- `sub_80E7260` reworked) with EXE6's marker (@exelib/lifesync/marker, effect object #0x60, EXE5's look). The usual
-- dimming phases, with an effect that marks the enemy two panels ahead and, after a warning over each, sets the HP
-- of the other combatants (side 1's, whoever uses it) to that enemy's, at most their own max HP (0x080E72C4; EXE6's
-- not capped). A link or boss-ranked battle (unless in the operation battle) turns it into a wait, as do finding no enemy
-- there and no one to sync (81 ticks: EXE6's waits otherwise). In the operation battle (never in a netbattle) the
-- enemy two panels ahead takes a hit of 50 instead (0x080E739C), then the wait.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local marker = require("@exelib/lifesync/marker")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local operation_battle = require("../../lib/operation_battle")

local lifesync = {}

export type State = {
    -- The enemy whose HP the others take (ExtraVars+0).
    target: Object?,
    -- The combatants that take it (ExtraVars+4 to +0x10, four slots).
    synced: { Object? },
    -- The marker (ExtraVars+0x14).
    marker: Object?,
}

local MARKER_TICKS = 0x50
-- The wait before the warning, the warning, after the sync, and the wait instead of it all.
local AIM, WARNING, AFTER, NOTHING = 0x14, 0x3C, 0x1E, 0x50
local SOUND_AIM = asset.sound("mark")
-- The sync's sound: the combatant's HP went up, or not.
local SOUND_UP = asset.sound("hub")
local SOUND_DOWN = asset.sound("lifesync")
local SOUND_WARNING = asset.sound("err-select-91")
-- The other side's player navi stands there (0x080E7394).
local ENEMY_NAVI = { [0] = 0x00200000, [1] = 0x00400000 }
-- The marker's attach point on its target's sprite.
local MARK_POINT = 0x1B
-- (The battle effects that make it only wait: 1, a boss rank; 8, a link battle.)


lifesync.marker = marker.make_kind("lifesync/marker", asset.sprite("hit-sparks"))

-- 0x080E7356 (EXE6's `sub_80E7486`): if the panel shows the other side's player navi, the first combatant (both
-- sides' slots) whose collision is on it.
local function combatant_on(x: number, y: number, side: number): Object?
    if not bit32.btest(field.flags(x, y), ENEMY_NAVI[side]) then
        return nil
    end
    for s = 0, 1 do
        for _, o in battle.alive_actors(s) do
            local c = o.collision
            if c.panel_x == x and c.panel_y == y then
                return o
            end
        end
    end
    return nil
end

-- Nothing to sync: 81 ticks, then done.
local function nothing(me: Object)
    me.timer = NOTHING
    me.phase, me.phase_init = 0xC, 0
end

-- 0x080E7174: the marker on the panel two ahead, the enemy there, the combatants to sync.
local function aim(me: Object)
    if not operation_battle() and (battle.boss_rank() or battle.link()) then
        return nothing(me)
    end
    local s = me.state :: State
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    local cx, cy = field.center(x, y)
    local m = marker.spawn(lifesync.marker, Vec3.new(cx, cy, 0), MARKER_TICKS)
    s.marker = m
    battle.play_sound(SOUND_AIM)
    if operation_battle() then
        -- 0x080E739C: a hit of 50 on the enemy there (0x04050001: an attack reaching navis, the plain spark).
        if combatant_on(x, y, me.alliance) then
            -- (`sub_80C53A6`'s counterpart: it runs while the battle is dimmed.)
            local h = battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = 0,
                z = 0,
                region = regions.single,
                hit_effect = sparks.plain,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = 50,
                hit_mod = 3,
            })
            if h then
                h.run_while_dimmed = true
            end
        end
        return nothing(me)
    end
    local target = combatant_on(x, y, me.alliance)
    if not target then
        return nothing(me)
    end
    s.target = target
    -- The marker over the target: its sprite's attach point (as drawn facing right) toward the user, and up.
    local ax, az = target:attach_point(MARK_POINT)
    ax *= target.facing
    if m then
        local p = target.pos
        m.pos = Vec3.new(p.x + ax * -me.facing * 0x1_0000, p.y, p.z + az * 0x1_0000)
    end
    -- (It lists side 1's slots whatever the user's side.)
    local n = 0
    for i = 1, 4 do
        s.synced[i] = nil
    end
    for _, o in battle.alive_actors(1) do
        if o ~= target then
            n += 1
            s.synced[n] = o
        end
    end
    if n == 0 then
        return nothing(me)
    end
    me.timer = AIM
    me.phase, me.phase_init = 4, 0
end

-- 0x080E725C: 21 ticks.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x080E726A: the warning over each combatant for 61 ticks, then the sync.
local function warn(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = WARNING
        me.phase_init = 4
    end
    for i = 1, 4 do
        local o = s.synced[i]
        if o then
            local c = o.collision
            local x, y = field.center(c.panel_x, c.panel_y)
            battle.warn(SOUND_WARNING, Vec3.new(x, y, o.pos.z))
        end
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    -- (The HP it gives is capped by each one's max HP in turn, and stays capped for those after.)
    local target = s.target :: Object
    local hp = target.hp
    for i = 1, 4 do
        local o = s.synced[i]
        if o then
            battle.play_sound(if o.hp < hp then SOUND_UP else SOUND_DOWN)
            hp = math.min(hp, o.max_hp)
            o.hp = hp
        end
    end
    me.timer = AFTER
    me.phase, me.phase_init = 0xC, 0
end

-- 0x080E7320: the rest, then done.
local function after(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

local function effect(me: Object)
    local phase = me.phase
    if phase == 0 then
        aim(me)
    elseif phase == 4 then
        wait(me)
    elseif phase == 8 then
        warn(me)
    elseif phase == 0xC then
        after(me)
    else
        error(string.format("LifeSync's controller: phase %#x reads past its table", phase))
    end
end

lifesync.controller = define.kind {
    id = "lifesync/controller",
    pool = "effect",
    state = { target = "object", synced = "object[4]", marker = "object" },
    update = dimming_chips.phases { name = "LifeSync", effect = effect },
}

-- 0x080E7334 (EXE6's `sub_80E7464`): LifeSync's `dimming` hook.
function lifesync.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(lifesync.controller, user, spec)
end

return lifesync
