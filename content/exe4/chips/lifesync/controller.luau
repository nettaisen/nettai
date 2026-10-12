--!strict
-- LifeSync's dimming controller (EXE4's effect object #0x5C, 0x080E8784; EXE5's #0x5C, 0x080E710C, is this code
-- reworked): the usual dimming phases (@exelib/dimming), with an effect (0x080E87C8, by phase) that, outside link
-- battles, marks the panel two ahead (@exelib/lifesync/marker, EXE4's #0x60: it stays on that panel) and, after a
-- warning over each, sets the HP of the combatants it lists but the enemy there to that enemy's (uncapped). It lists
-- BattleState +0x44's entries past the first (lib/navi_list: the other console's navi, none past it), whatever the
-- user's side. A link battle (every netbattle: the lab's chips/0x01c-lifesync) turns it into a wait of 81 ticks, with
-- no marker; so do finding no enemy there and no one to sync, after the marker. Under event flag 0x1187
-- (lib/main_started: never in a netbattle) the link battle doesn't stop it, and the enemy navi there takes a hit of 50
-- instead (0x080E8A48), then the wait: EXE5's operation battle's.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local main_started = require("../../lib/main_started")
local marker = require("@exelib/lifesync/marker")
local navi_list = require("../../lib/navi_list")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local lifesync = {}

export type State = {
    -- The enemy whose HP the others take (+0x60).
    target: Object?,
    -- The combatants that take it (+0x64, +0x68; and a third it can write, +0x6C, which nothing reads).
    synced: { Object? },
}

local MARKER_TICKS = 0x50
-- The wait before the warning, the warning, after the sync, and the wait instead of it all.
local AIM, WARNING, AFTER, NOTHING = 0x14, 0x3C, 0x1E, 0x50
-- The slots the warning and the sync read.
local SLOTS = 2
local SOUND_AIM = asset.sound("mark")
-- The sync's sound: the combatant's HP went up (ColorPt's point's), or not.
local SOUND_UP = asset.sound("point-arrive")
local SOUND_DOWN = asset.sound("lifesync")
-- The warning's (0x08008424).
local SOUND_WARNING = asset.sound("program-advance-part")
-- The other side's player navi stands there (0x080E89F8, 0x080E8A40).
local ENEMY_NAVI = { [0] = 0x00200000, [1] = 0x00400000 }
-- Its hit under event flag 0x1187.
local HIT_DAMAGE, HIT_MOD = 50, 3

-- 0x080E8C98: in the hit sparks, palette 7, 10 pixels over its panel.
lifesync.marker = marker.make_kind {
    id = "lifesync/marker",
    sprite = asset.sprite("hit-sparks"),
    palette = 7,
    height = 0xA_0000,
}

-- 0x080E89BA: if the panel shows the other side's player navi, the first of BattleState +0x44's list whose collision
-- is on it.
local function combatant_on(x: number, y: number, side: number): Object?
    if not bit32.btest(field.flags(x, y), ENEMY_NAVI[side]) then
        return nil
    end
    for i = 0, navi_list.LENGTH - 1 do
        local o = navi_list.at(i)
        if o then
            local c = o.collision
            if c.panel_x == x and c.panel_y == y then
                return o
            end
        end
    end
    return nil
end

-- 0x080E8A00: if the panel shows the other side's player navi, that navi (BattleState +0x44's entry by its side,
-- `battle_networkInvert`) when its collision is on it.
local function enemy_navi_on(x: number, y: number, side: number): Object?
    if not bit32.btest(field.flags(x, y), ENEMY_NAVI[side]) then
        return nil
    end
    local o = battle.player(bit32.bxor(side, 1))
    if o then
        local c = o.collision
        if c.panel_x == x and c.panel_y == y then
            return o
        end
    end
    return nil
end

-- 0x080E8A48 (under event flag 0x1187): a hit of 50 on (x, y) (0x04050001: an attack reaching navis, the plain
-- spark), running while dimmed (0x080CD81E).
local function hit(me: Object, x: number, y: number)
    local h = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = HIT_DAMAGE,
        hit_mod = HIT_MOD,
    })
    if h then
        h.run_while_dimmed = true
    end
end

-- Nothing to sync: 81 ticks, then done.
local function nothing(me: Object)
    me.timer = NOTHING
    me.phase, me.phase_init = 0xC, 0
end

-- 0x080E87EC: in a link battle (0x08007EEC's battle flags' 8, or a battle type of 70 and on, 0x08007E46: the
-- netbattles' 0x48), unless event flag 0x1187 is set, nothing. Else the marker on the panel two ahead (by its side,
-- `object_getEnemyDirection`), the enemy there, the combatants to sync.
local function aim(me: Object)
    if not main_started and battle.link() then
        return nothing(me)
    end
    local s = me.state :: State
    local x, y = me.panel_x + 2 * (1 - 2 * me.alliance), me.panel_y
    -- 0x080E8CFE: on that panel (its position the spawner's registers, which its init replaces).
    local cx, cy = field.center(x, y)
    local m = marker.spawn(lifesync.marker, Vec3.new(cx, cy, 0), MARKER_TICKS)
    if m then
        m.panel_x, m.panel_y = x, y
    end
    battle.play_sound(SOUND_AIM)
    if main_started then
        if enemy_navi_on(x, y, me.alliance) then
            hit(me, x, y)
        end
        return nothing(me)
    end
    local target = combatant_on(x, y, me.alliance)
    if not target then
        return nothing(me)
    end
    s.target = target
    s.synced[1], s.synced[2] = nil, nil
    local n = 0
    for i = 1, navi_list.LENGTH - 1 do
        local o = navi_list.at(i)
        if o and o ~= target then
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

-- 0x080E88A4: 21 ticks.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x080E88B2: (the warning's graphics queued) the warning over each of the two for 61 ticks, then the sync, each
-- taking the target's HP.
local function warn(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = WARNING
        me.phase_init = 4
    end
    for i = 1, SLOTS do
        local o = s.synced[i]
        if o then
            local c = o.collision
            local x, y = field.center(c.panel_x, c.panel_y)
            battle.warn(SOUND_WARNING, Vec3.new(x, y, o.pos.z))
        end
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t >= 0 then
        return
    end
    local hp = (s.target :: Object).hp
    for i = 1, SLOTS do
        local o = s.synced[i]
        if o then
            battle.play_sound(if o.hp < hp then SOUND_UP else SOUND_DOWN)
            o.hp = hp
        end
    end
    me.timer = AFTER
    me.phase, me.phase_init = 0xC, 0
end

-- 0x080E8984: the rest, then done.
local function after(me: Object)
    local t = me.timer - 1
    me.timer = int.u16(t)
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

lifesync.controller = new.kind {
    id = "lifesync/controller",
    pool = "effect",
    state = { target = "object", synced = "object[3]" },
    -- 0x080E8784.
    update = dimming_chips.phases { name = "LifeSync", effect = effect },
}

-- 0x080E8998: LifeSync's `dimming` hook (only the alliance byte of its user's).
function lifesync.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(lifesync.controller, user, spec, true)
end

return lifesync
