--!strict
-- DeathPhoenix, DethPhnx's (EXE5's actor object #0x28, 0x080BFD98; EXE5's own), brought by its controller
-- (../phoenix/controllers). It appears at the back corner of its user's side (column 1 or 6, row 3) and, as it comes,
-- lists for each row four of the solid panels ahead of it (each row's shuffled, repeated as needed). After a hit of
-- no damage over the whole field it fades in, waits 30 ticks, then sets 12 fires (./fire), 12 ticks apart: four
-- rounds over the three rows in an order drawn each round, each fire on the round's panel of its row's list. 40
-- ticks on it fades out; then, if its user is alive and a navi chip was used this battle, it brings that chip's navi
-- again for its user (the user warps out first and back after, unless that navi brings the user back itself), and
-- the controller moves on.
--
-- Not reached: the palette flash it would show with each fire when bit 19 of its update's r7 is set (0x080BFF2E),
-- which holds its list node's address, an EWRAM address under 0x02040000.

local controllers = require("../phoenix/controllers")
local fire = require("./fire")
local collision = require("../../rules/collision")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local phoenix = {}

export type State = {
    -- The controller waiting on it (+0x54: the flag it was given).
    controller: Object?,
    -- Each row's four panels (+0x6C, +0x70, +0x74), by (row - 1) * 4 + the round.
    rows: { number },
    -- The round's row order (+0x68, into ORDERS), the fire within the round (+0x60), the round (+0x78).
    order: number,
    fire: number,
    round: number,
    -- The navi it brought again is acting (+0x18).
    navi_acting: boolean,
}

local SPRITE = asset.sprite("phoenix")
local SOUND_APPEAR = asset.sound("appear")
-- Its animation (DethPhnx's; Phoenix's is 0).
local ANIM = 1
local SOLID = 0x10
-- The rows' orders, a round's draw picking among the first three (0x080BFF50).
local ORDERS = { 1, 2, 3, 1, 3, 2, 2, 1, 3, 2, 3, 1, 3, 1, 2, 3, 2, 1 }
local FADE = 0x10
local FIRES, FIRE_EVERY = 0xC, 0xC

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- 0x080C0162: the solid panels of `row` from the column ahead of it to the field's end, copied and shuffled (as
-- many swaps as panels) until there are four; four zeros with none.
local function list_row(me: Object, row: number)
    local s = me.state :: State
    local found: { number } = {}
    local x = me.panel_x + me.facing
    while field.valid(x, row) do
        if field.check(x, row, SOLID, 0) then
            local p: number = panels.pack(x, row)
            table.insert(found, p)
        end
        x += me.facing
    end
    local list: { number } = {}
    if #found > 0 then
        while #list < 4 do
            local copy = table.clone(found)
            panels.shuffle(copy, #found, #found)
            for _, p in copy do
                table.insert(list, p)
            end
        end
    end
    for i = 1, 4 do
        s.rows[(row - 1) * 4 + i] = list[i] or 0
    end
end

-- 0x080C0128: a fire on (x, y) (none for x 0): far from it when 3 columns or more away, at 3 columns one draw in
-- two (bit 4 of an RNG draw clear).
local function set_fire(me: Object, x: number, y: number)
    if x == 0 then
        return
    end
    local d = math.abs(me.panel_x - x)
    local far = false
    if d > 2 then
        far = d >= 4 or bit32.band(battle.rng_positive(), 0x10) == 0
    end
    fire.spawn(me, x, y, far, damage_word(me))
end

-- Its controller moves on, and it goes.
local function leave(me: Object)
    local c = (me.state :: State).controller
    if c then
        controllers.done(c)
    end
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080BFE4C): a hit of no damage over the whole field from panel (1, 1), running while dimmed (region
-- 0x80, no hit effect, target 5, self 0x27, hit modifier 0); then the fade-in.
local function field_hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = 1,
        panel_y = 1,
        element = 0,
        z = 0,
        region = regions.field,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.collision_27,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
    })
    if box then
        box.run_while_dimmed = true
    end
    me:set_action(4)
end

-- Action 4 (0x080BFE6A): shown, its sound; alpha 1 to 15 a tick; then solid.
local function fade_in(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
    end
    local t = int.u16(me.timer + 1)
    me.timer = t
    if t >= FADE then
        me.sprite.alpha = nil
        me:set_action(8)
        return
    end
    me.sprite.alpha = t
end

-- Action 8 (0x080BFEA8): 30 ticks; then the fires from the first.
local function ready(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.timer, me.timer2 = 0, 0
    s.fire, s.round = 0, 0
    me:set_action(0xC)
end

-- Action 0xC (0x080BFED8): a fire every 12 ticks (the first at once): a round's first draws its row order (RNG % 3);
-- each fire is on the round's panel of its row. After the twelfth, action 0x10.
local function fires(me: Object)
    local s = me.state :: State
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.timer = FIRE_EVERY
    if s.fire == 0 then
        s.order = battle.rng_positive() % 3
    end
    local row = ORDERS[s.order * 3 + s.fire + 1]
    local p = s.rows[(row - 1) * 4 + s.round + 1]
    set_fire(me, bit32.band(p, 7), bit32.rshift(p, 4))
    s.fire += 1
    if s.fire >= 3 then
        s.fire = 0
        s.round += 1
    end
    me.timer2 += 1
    if me.timer2 >= FIRES then
        me:set_action(0x10)
    end
end

-- Action 0x10 (0x080BFF62): 40 ticks.
local function linger(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x28
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(0x14)
    end
end

-- Action 0x14 (0x080BFF84): alpha 15 down to 0 a tick; then hidden, and the last navi chip's navi again (action
-- 0x18) if the user is alive and a navi chip was used, else it is done.
local function fade_out(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.timer = FADE
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        me.sprite.alpha = left
        return
    end
    me.visible = false
    me.sprite.alpha = nil
    local user = me.related1 :: Object
    if user.hp ~= 0 and navi_chip.last() then
        me:set_action(0x18)
    else
        leave(me)
    end
end

-- 30 ticks from the phase's first, then `next`.
local function wait(me: Object, next: () -> ())
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        next()
    end
end

-- Action 0x18 (0x080BFFDE): the last navi chip's navi again, as the navi chip's controller brings it (0x080E1854
-- to 0x080E18F8), with itself in the controller's place: the user warps out (31 ticks); the navi, at the user's
-- panel with the chip's element and damage (bonus included), until it leaves; 31 ticks; the user warps back in (31
-- ticks), unless the navi brings it back itself (Roll's, navi 25).
local function bring_again(me: Object)
    local s = me.state :: State
    local user = me.related1 :: Object
    local phase = me.phase
    if phase == 0 then
        if me.phase_init == 0 then
            navi_chip.warp(user, true)
        end
        wait(me, function()
            me.phase, me.phase_init = 4, 0
        end)
    elseif phase == 4 then
        if me.phase_init == 0 then
            local last = navi_chip.last()
            if not last then
                error("DethPhnx brings the last navi chip's navi, but none was used")
            end
            local bring = last.chip.navi :: (Object, Object, NaviChipSpec) -> Object?
            local navi = bring(user, me, {
                panel_x = user.panel_x,
                panel_y = user.panel_y,
                element = last.element,
                damage = last.damage,
            })
            s.navi_acting = navi ~= nil
            me.phase_init = 4
        end
        if not s.navi_acting then
            me.phase, me.phase_init = 8, 0
        end
    elseif phase == 8 then
        wait(me, function()
            me.phase, me.phase_init = 0xC, 0
        end)
    elseif phase == 0xC then
        if me.phase_init == 0 then
            local last = navi_chip.last()
            if last and table.find(last.chip.traits or {}, "navi_returns_user") then
                return leave(me)
            end
            navi_chip.warp(user, false)
        end
        wait(me, function()
            leave(me)
        end)
    else
        error(string.format("DethPhnx's phoenix: phase %#x of action 0x18 reads past its table (0x080BFFF0)", phase))
    end
end

local ACTIONS = {
    [0] = field_hit,
    [4] = fade_in,
    [8] = ready,
    [0xC] = fires,
    [0x10] = linger,
    [0x14] = fade_out,
    [0x18] = bring_again,
}

-- 0x080BFE1C.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("DethPhnx's phoenix: action %#x reads past its table (0x080BFE30)", me.action))
    end
    action(me)
end

-- 0x080BFDBC: its sprite (animation 1, no shadow), facing its side's way, at the back corner of its side (column 1
-- or 6, row 3), 3 pixels down the field and 3 up; each row's list. Hidden until its fade-in.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, 0xFF
    local facing = bit32.bxor(me.alliance, me.flip)
    sprite:set_flip(facing)
    me.panel_x, me.panel_y = facing * 5 + 1, 3
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + 0x3_0000, 0x3_0000)
    for row = 1, 3 do
        list_row(me, row)
    end
    me:set_lifecycle("update")
end

phoenix.kind = new.kind {
    id = "dethphnx/phoenix",
    pool = "actor",
    state = {
        controller = "object",
        rows = "u8[12]",
        order = "u8",
        fire = "u8",
        round = "u8",
        navi_acting = "bool",
    },
    -- 0x080BFD98.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
    -- The navi it brought again is done.
    navi_left = function(me: Object)
        (me.state :: State).navi_acting = false
    end,
}

-- 0x080C00B6: DeathPhoenix for `user`, on `owner`'s panel (its controller's), of the user's side and facing, with
-- the damage word. (Its position is the spawner's registers, which its init overwrites.)
function phoenix.spawn(owner: Object, user: Object, damage: number): Object?
    local o = battle.spawn(phoenix.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.element = owner.element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        local s = o.state :: State
        s.controller = owner
    end
    return o
end

return phoenix
