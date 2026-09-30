--!strict
-- TenguMan's own chip, FTornado (action 0x0A for AI index 8,
-- `sub_80F1A46`): he fans, and a Tengu tornado (attack object #0x60,
-- objects/tengu-tornado) rises on every panel ahead of him with an enemy
-- (or a neutral object) on it, with a flash (effect 0x41) over him and
-- sound 0xB8; 31 ticks, then 21 of recovery. [unverified: no lab scenario
-- reaches it; docs/engine/standard-chips.md]

local tengu_tornado = require("../../objects/tengu-tornado/tengu_tornado")
local link = require("../190-heatpres/state")
type State = LinkChipState

local routine = {}

local FAN, RECOVER = 0x0, 0x4
-- The panels it strikes, by side (`byte_80F1B70`): the other side's
-- bodies, and neutral objects.
local TARGETS = { [0] = 0x0580_0000, 0x0A80_0000 }
-- The tornadoes are spawned with this in the Z register (the table's
-- address), whose fraction they keep.
local TABLE_ADDRESS = 0x080F_1B70
local FLASH, FLASH_Z = 0x41, 0x20_0000

-- `sub_80F1B24`: a tornado on each panel ahead with a target on it, row
-- by row from the top.
local function tornadoes(me: Object)
    local targets = TARGETS[me.alliance]
    for y = 1, 3 do
        for x = 1, 6 do
            if me.facing * (me.panel_x - x) < 0 and bit32.btest(field.flags(x, y), targets) then
                tengu_tornado.spawn(me, x, y, me.attack_element, link.damage_word(me), TABLE_ADDRESS)
            end
        end
    end
end

-- `sub_80F1B78`: a whole-field region (0x80, self type 1, target 2) on
-- panel (0, 0), with no damage; off the field, it frees itself at once.
local function empty_hit(me: Object)
    battle.hitbox(me, {
        panel_x = 0,
        panel_y = 0,
        element = 0,
        region = 0x80,
        hit_effect = 0xFF,
        target = 2,
        self_type = 1,
        damage = 0,
    })
end

-- `sub_80F1A60`: animation 0x10, the tornadoes; 31 ticks.
local function fan(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0x10
        link.add_bonus(me)
        tornadoes(me)
        empty_hit(me)
        battle.play_sound(0xB8)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, FLASH_Z), FLASH, bit32.bxor(me.alliance, me.flip))
        s.timer = 0x1E
    elseif link.count_down(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1AB6`: animation 7 for 21 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 7
        s.timer = 0x14
    elseif link.count_down(s) then
        me.anim = 0
        me:exit_attack()
    end
end

-- `sub_80F1A46` (`off_80F1A58`).
function routine.update(me: Object, s: State)
    local step = me.step
    if step == FAN then
        fan(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("FTornado phase %#x reads past off_80F1A58", step))
    end
end

return routine
