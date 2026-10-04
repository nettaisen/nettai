--!strict
-- ChargeMan, the navi of the ChrgeMan chips (the original's actor object
-- #0x16, `sub_80BB914`), brought by their navi chip controller. He appears
-- with his train: a car on each free, solid panel behind him (up to five,
-- ./car), then charges ahead at 4 pixels a tick to his side's
-- far edge, hitting each new panel he reaches, his cars following 60
-- ticks later; he leaves where the floor ends, or vanishes off the field.
-- Then the controller moves on. See docs/engine/chips.md §3.6.17.

local car = require("./car")
local navi_chips = require("../../lib/navi-chips/navi")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
    -- How many cars he brought (ExtraVars[0], which nothing reads).
    cars: number,
}

local chrgeman = {}

local SPRITE = asset.sprite("bn6:chargeman")
local SOUND_APPEAR = asset.sound("bn6:appear")
local SOUND_CHARGE_MAN = asset.sound("bn6:charge-man")

-- A car's panel: solid, and none of these (`dword_80BBB8C`).
local CAR_REQUIRE, CAR_FORBID = 0x10, 0x0F80_0000

-- His and his cars' speed (16.16), and how long the cars wait.
local SPEED = 0x4_0000
local CAR_WAIT = 0x3C

-- `sub_80BB938`: on his panel, on the ground.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me:set_lifecycle("update")
end

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- `sub_80BB99C`: he appears; 3 ticks; on, or straight to leaving if his
-- panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer, me.timer2 = 3, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then 4 else 0xC)
end

-- `sub_80BBB38`: his cars, one on each panel behind him until one can't
-- take a car (five at most); how many.
local function cars(me: Object): number
    local n = 0
    while n <= 4 do
        local x, y = me.panel_x - me.facing * (n + 1), me.panel_y
        if not field.check(x, y, CAR_REQUIRE, CAR_FORBID) then
            return n
        end
        -- r3 (0xFF << 24) becomes the car's Z.
        car.spawn(me, x, y, me.element, { while_dimmed = true, place = n, wait = CAR_WAIT }, damage_word(me), SPEED, 0xFF00_0000)
        n += 1
    end
    return n
end

-- `sub_80BB9F0`: his cars appear; 60 ticks (at 30 left he gets ready).
local function hitch(me: Object)
    if me.phase == 0 then
        me.phase = 4;
        (me.state :: State).cars = cars(me)
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x1E then
        me.anim = 0xF
        return
    end
    if left <= 0 then
        me:set_action(8)
    end
end

-- The hit on his panel, running while dimmed (`sub_80BBB18`,
-- `sub_80C53A6`).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- The panel under a position (`sub_800E258`: two BIOS divisions).
local function panel_under(p: Vec3): (number, number)
    return int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28), int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
end

-- `sub_80BBA26`: the charge (`sub_80BBAEC`: |edge - x| panels of
-- 40 pixels at his speed, a BIOS division), hitting each new panel; he
-- leaves where the floor ends, and vanishes once he is off the field.
local function charge(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_CHARGE_MAN)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        local edge = if me.alliance == 0 then 7 else 0
        me.timer = int.tdiv(math.abs(edge - me.panel_x) * 0x28_0000, SPEED)
        return
    end
    local old_x = me.panel_x
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    local x, y = panel_under(me.pos)
    if field.valid(x, y) then
        if not field.solid(x, y) then
            me:set_action(0xC)
            return
        end
        me:set_panel_from_coordinates()
        if me.panel_x ~= old_x then
            hit(me)
        end
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0x10)
    end
end

-- The controller moves on, and he goes.
local function go(me: Object)
    navi_chips.leave(me, (me.state :: State).controller)
end

-- `sub_80BBA8E`: 4 ticks (to -1), then he goes.
local function leave(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 4
        me.timer, me.timer2 = 3, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        go(me)
    end
end

-- `sub_80BBAB8`: off the field: he goes at once.
local function vanish(me: Object)
    go(me)
end

local ACTIONS = { [0] = appear, [4] = hitch, [8] = charge, [0xC] = leave, [0x10] = vanish }

chrgeman.kind = define.kind {
    id = "bn6:chrgeman/navi",
    pool = "actor",
    state = { controller = "object", cars = "u32" },
    -- `sub_80BB914`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("ChargeMan action %#x reads past his table (off_80BB988)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80BBAC6`: ChargeMan for `user`, reporting to `controller` (the
-- ChrgeMan chips' navi hook, `off_802CD5C[6]`). (His position is the
-- spawner's registers, all of which his init overwrites.)
function chrgeman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(chrgeman.kind, user, spec)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return chrgeman
