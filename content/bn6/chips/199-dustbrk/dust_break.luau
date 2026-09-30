--!strict
-- DustMan's own chip, DustBrk (action 0x0A for AI index 10,
-- `sub_80F1FA0`): he sucks (31 ticks, the suction cloud on him, effect
-- object #0x72): every tick a zero-damage suction hit (self type 0x1E) on
-- each enemy panel of his row; then he blows (the blow cloud, #0x71): 10
-- ticks in, one hit on the panel ahead; 40 ticks, then 30 of recovery.
-- [unverified: no lab scenario reaches it; docs/engine/standard-chips.md]

local dust_cloud = require("../../objects/dust-cloud/dust_cloud")
local link = require("../190-heatpres/state")
type State = LinkChipState

local routine = {}

local SUCK, BLOW, RECOVER = 0x0, 0x4, 0x8
-- The enemy's panels by side (`byte_80F20E0`): require, forbid.
local ENEMY_AREA = { [0] = { 0x20, 0 }, [1] = { 0, 0x20 } }
-- The suction: region 1, no spark, target 5, self type 0x1E, no damage,
-- hit modifier 0x10.
local SUCTION = { region = 1, hit_effect = 0xFF, target = 5, self_type = 0x1E, hit_mod = 0x10 }
-- The blow: region 1, spark 0xA, target 5, self type 6, hit modifier 3.
local BLOW_HIT = { region = 1, hit_effect = 0x0A, target = 5, self_type = 6, hit_mod = 3 }
local BLOW_AT = 0x1E

-- `sub_80F20A0`: the suction on each enemy panel of his row.
local function suction(me: Object)
    local area = ENEMY_AREA[me.alliance]
    local y = me.panel_y
    for x = 1, 6 do
        if field.check(x, y, area[1], area[2]) then
            battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = 0,
                region = SUCTION.region,
                hit_effect = SUCTION.hit_effect,
                target = SUCTION.target,
                self_type = SUCTION.self_type,
                damage = 0,
                hit_mod = SUCTION.hit_mod,
            })
        end
    end
end

-- `sub_80F1FC0`: animation 0x12, the suction cloud, sound 0xAD; 31 ticks,
-- the suction every tick.
local function suck(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = 0x12
        s.held = dust_cloud.spawn(me, "dust-suction", { 0, 0xA })
        battle.play_sound(0xAD)
        s.timer = 0x1E
    elseif link.count_down(s) then
        link.go(me, BLOW)
    end
    suction(me)
end

-- `sub_80F207C`: the blow on the panel ahead.
local function blow_hit(me: Object)
    local d = link.damage_word(me)
    battle.hitbox(me, {
        panel_x = me.panel_x + me.facing,
        panel_y = me.panel_y,
        element = me.attack_element,
        region = BLOW_HIT.region,
        hit_effect = BLOW_HIT.hit_effect,
        target = BLOW_HIT.target,
        self_type = BLOW_HIT.self_type,
        damage = d,
        stamina = bit32.rshift(d, 16),
        hit_mod = BLOW_HIT.hit_mod,
    })
end

-- `sub_80F2004`: animation 0x14, the blow cloud (the suction's goes); the
-- blow 10 ticks in, sound 0x17B; 40 ticks.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0x14
        s.held2 = dust_cloud.spawn(me, "dust-blow", { 0xA })
        s.timer = 0x28
        local cloud = s.held
        if cloud then
            dust_cloud.stop(cloud)
        end
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == BLOW_AT then
        blow_hit(me)
        battle.play_sound(0x17B)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F204E`: 30 ticks, then idle, and the blow cloud goes.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = 0x1E
    elseif link.count_down(s) then
        me.anim = 0
        me:exit_attack()
        local cloud = s.held2
        if cloud then
            dust_cloud.stop(cloud)
        end
    end
end

-- `sub_80F1FA0` (`off_80F1FB4`).
function routine.update(me: Object, s: State)
    local step = me.step
    if step == SUCK then
        suck(me, s)
    elseif step == BLOW then
        blow(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("DustBrk phase %#x reads past off_80F1FB4", step))
    end
end

return routine
