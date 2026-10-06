--!strict
-- LifeSync's dimming controller (the original's effect object #0x5C,
-- `sub_80E7260`; docs/engine/dimming-chip-effects.md §14): the usual
-- dimming phases (@exelib/dimming), with an effect that, outside link
-- battles, marks the enemy two panels ahead and after a warning sets the
-- HP of the other side's other combatants to that enemy's.
--
-- In a link battle (every PvP battle) the effect jumps into the middle of
-- the HP sync with nothing to sync: it lasts the ticks the dim left on the
-- timer, plus one, then 31 more.

local dimming_chips = require("@exelib/dimming")
local marker = require("./marker")

local controller = {}

export type State = {
    -- The enemy whose HP the others take (ExtraVars+0).
    target: Object?,
    -- The combatants that take it (ExtraVars+4 to +0x10, four slots).
    synced: { Object? },
}

-- The marker's lifetime, and the ticks before the warning, of the warning
-- and after the sync.
local MARKER_TICKS = 0x50
local AIM, WARNING, AFTER = 0x14, 0x3C, 0x1E
local SOUND_AIM = asset.sound("mark")
local SOUND_WARNING = asset.sound("err-select-91")
-- The sync's sound: the combatant's HP went up, or not.
local SOUND_UP = asset.sound("hub")
local SOUND_DOWN = asset.sound("lifesync")
-- `byte_80E74C4`: the other side's player navi stands there.
local ENEMY_NAVI = { [0] = 0x00200000, [1] = 0x00400000 }
-- The virus that LifeSync neither takes nor gives HP to: AI 13, all its
-- versions (NameIDs 0x49 to 0x4E).
local IMMUNE_AI = 13
-- The marker's attach point on its target's sprite.
local MARK_POINT = 0x1B

-- NameIDs 0x49 to 0x4E.
local function immune(o: Object): boolean
    return o.actor_type == "virus" and o.ai_index == IMMUNE_AI
end

-- `sub_80E7486`: if the panel shows the other side's player navi, the
-- first combatant (both sides' slots) whose collision is on it.
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

-- `loc_80E73C4` (in `sub_80E73A2`): a warning over each synced combatant
-- (over its collision's panel, at its height), and when the timer runs
-- out, the sync: each takes the target's HP. The link
-- battles' path and the failed aims come here with nothing to sync (the
-- original reads the target's HP through a null pointer then, and uses it
-- for nothing).
local function warn_and_sync(me: Object)
    local s = me.state :: State
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
    local target = s.target
    if target then
        local hp = target.hp
        for i = 1, 4 do
            local o = s.synced[i]
            if o and o.active then
                battle.play_sound(if o.hp < hp then SOUND_UP else SOUND_DOWN)
                o.hp = hp
            end
        end
    end
    me.timer = AFTER
    me.phase, me.phase_init = 0xC, 0
end

-- `sub_80E72C8`: in a link (or boss-ranked) battle, straight to the sync's
-- tail. Else the marker on the panel two ahead and the enemy there (none,
-- or the immune virus: the sync's tail, and the same again next tick); it
-- moves onto the enemy, and the other side's other combatants are listed
-- (none: the sync's tail); then 21 ticks to the warning.
local function aim(me: Object)
    if battle.link() or battle.boss_rank() then
        warn_and_sync(me)
        return
    end
    local s = me.state :: State
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    local cx, cy = field.center(x, y)
    local m = marker.spawn(Vec3.new(cx, cy, 0), MARKER_TICKS)
    battle.play_sound(SOUND_AIM)
    local target = combatant_on(x, y, me.alliance)
    if not target or immune(target) then
        warn_and_sync(me)
        return
    end
    s.target = target
    -- The marker over the target: its sprite's attach point (as drawn
    -- facing right) toward the user, and up.
    local ax, az = target:attach_point(MARK_POINT)
    ax *= target.facing
    if m then
        local p = target.pos
        m.pos = Vec3.new(p.x + ax * -me.facing * 0x1_0000, p.y, p.z + az * 0x1_0000)
    end
    -- sub_80E72C8 lists side 1's slots whatever the user's side.
    local n = 0
    for i = 1, 4 do
        s.synced[i] = nil
    end
    for _, o in battle.alive_actors(1) do
        if not immune(o) and o ~= target then
            n += 1
            s.synced[n] = o
        end
    end
    if n == 0 then
        warn_and_sync(me)
        return
    end
    me.timer = AIM
    me.phase, me.phase_init = 4, 0
end

-- `sub_80E7394`: 21 ticks.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80E73A2`: (the warning's graphics loaded) the warning for 61
-- ticks, then the sync.
local function warn(me: Object)
    if me.phase_init == 0 then
        me.timer = WARNING
        me.phase_init = 4
    end
    warn_and_sync(me)
end

-- `sub_80E7450`: 31 ticks after the sync.
local function after(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
    end
end

-- `sub_80E72A4`: by phase.
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
        error(string.format("LifeSync's controller: phase %#x reads past off_80E72B8", phase))
    end
end

controller.kind = new.kind {
    id = "lifesync/controller",
    pool = "effect",
    state = { target = "object", synced = "object[4]" },
    -- `sub_80E7260`.
    update = dimming_chips.phases { name = "LifeSync", effect = effect },
}

-- `sub_80E7464`: LifeSync's `dimming` hook.
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(controller.kind, user, spec)
end

return controller
