--!strict
-- MagnetMan, the navi of BN5's MagnetMan chips (BN5's actor object #0x0D, 0x080BAF2C; BN5's own), brought by their
-- navi chip controller: unseen himself, he works through two magnets (./magnet). Off the field he goes at once.
-- Else he hits his own panel lightly (a one-tick hit of 1, hitting navis), finds his target, the first panel
-- ahead along his row holding an enemy navi, and sends the first magnet (pole 0) from where he stands; 30 ticks
-- on, when the panel as far past the target as he is short of it is on the field and free, the second (pole 1)
-- from there, the two meeting at the target. 30 ticks on (or 20 more with one) he waits 31 more, and the
-- controller moves on.

local collision = require("../../rules/collision")
local magnet = require("./magnet")
local navi_chips = require("@exelib/navi-chips/navi")
local regions = require("@exelib/regions")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x60).
    controller: Object?,
    -- The chip's level (its Param1: 0, 4 for DS), which picks the magnets' palette.
    level: number,
    -- His target's column (+7: 0 for none), and the panel past it the second magnet comes from (+6).
    target_x: number,
    mirror_x: number,
    -- There is a second magnet (+0x64).
    paired: boolean,
}

local magnetman = {}

-- A panel holding an enemy navi's body, by side (0x080BB100), and one something stands on (as words).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local TAKEN = 0x0F80_0000

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- `object_isValidPanel`: a battle panel (columns 1 to 6, rows 1 to 3), whatever the number.
local function on_field(x: number, y: number): boolean
    return x >= 1 and x <= 6 and y >= 1 and y <= 3
end

-- `object_getFirstPanelInDirectionFiltered` (0x080BB0E0): from his own panel on along his row, the first column
-- holding an enemy navi; 0 if none before the field's end.
local function find_target(me: Object): number
    local x, y = me.panel_x, me.panel_y
    local need = ENEMY_NAVI[me.alliance]
    while true do
        if field.check(x, y, need, 0) then
            return x
        end
        x += me.facing
        if not on_field(x, y) then
            return 0
        end
    end
end

-- A magnet of pole `pole` from column `x`.
local function send(me: Object, x: number, pole: number)
    local s = me.state :: State
    magnet.spawn(me, x, me.panel_y, me.element, {
        pole = pole,
        paired = s.paired,
        target_x = s.target_x,
        level = s.level,
        damage = damage_word(me),
    })
end

-- Action 0 (0x080BAFE4): the first magnet, from his panel; 31 ticks; then the second, or none.
local function first(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        send(me, me.panel_x, 0)
        me.timer, me.timer2 = 0x1E, 0
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me:set_action(if (me.state :: State).paired then 4 else 8)
end

-- Action 4 (0x080BB030): the second magnet, from the panel past the target; 31 ticks.
local function second(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        send(me, (me.state :: State).mirror_x, 1)
        me.timer, me.timer2 = 0x1E, 0
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080BB070): 21 ticks (the magnets go on his action 8).
local function hold(me: Object)
    local left = navi_chips.countdown_on_init(me, 0x14)
    if left ~= nil and left < 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BB094): 31 ticks; then the controller moves on, and he goes.
local function leave(me: Object)
    local left = navi_chips.countdown_on_init(me, 0x1E)
    if left ~= nil and left < 0 then
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = first, [4] = second, [8] = hold, [0xC] = leave }

-- 0x080BAF50: off the field he goes at once. Else his target, the panel past it and whether a magnet can come from
-- there (on the field, no one on it); a light hit on his own panel.
local function init(me: Object)
    local s = me.state :: State
    s.paired = false
    if not on_field(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
        return
    end
    local target = find_target(me)
    s.target_x = int.u8(target)
    local mirror = s.target_x + (s.target_x - me.panel_x)
    s.mirror_x = int.u8(mirror)
    s.paired = on_field(mirror, me.panel_y) and field.check(s.mirror_x, me.panel_y, 0, TAKEN)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = 1,
        stamina = 0,
        hit_mod = 0,
    })
    me:set_lifecycle("update")
end

magnetman.kind = define.kind {
    id = "magnetmn/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", target_x = "u8", mirror_x = "u8", paired = "bool" },
    -- 0x080BAF2C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("MagnetMan action %#x reads past his table (0x080BAFD4)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BB0BA: MagnetMan for `user`, reporting to `controller` (the summon table's entry 8), of a chip of `level`.
function magnetman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        -- (His Z is the address the controller called him through, which nothing of his overwrites.)
        local o = navi_chips.spawn(magnetman.kind, user, spec, 0x080BB0BB)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- MagnetMan's and MagnetManSP's (their Param1s 0), MagnetManDS's (4).
magnetman.summon = magnetman.summon_at(0)
magnetman.summon_sp = magnetman.summon_at(0)
magnetman.summon_dark = magnetman.summon_at(4)

return magnetman
