--!strict
-- NumberMan's DiceBons (EXE5 DS's party battle: 0x02004CC8, from the chip attack's end 0x0200634C, `sub_801171C`'s
-- counterpart; his controller the DS's effect object 0x95, 0x02226488, his actor its actor object 0x33, 0x021FF938;
-- docs/design/exe5ds-map.md §4.7), the rules' `chip_ended`: as a navi's chip attack ends, a side that passes the light
-- test draws its chance (RNG2, every such end while the battle goes on); then, once a battle, with NumberMan a benched
-- member who may support (offline, side 0's alone), a chip without dimming just used and a chip that deals damage next,
-- the battle dims (the other side can't cut in), DiceBons' telop shows and NumberMan comes behind the navi and throws
-- two dice at the enemy area; the faces of those that land where he aimed, times ten, are added to the next chip's
-- attack.

local dice = require("../../chips/numbrman/dice")
local chips_dicebons = require("../../chips/dicebons")
local effects = require("../../lib/effects")
local numberman = require("../numberman").numberman
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")

local dice_bonus = {}

-- RNG2 bits 10–13 at most this: 7 in 16.
local CHANCE = 6
local DICEBONS = chips_dicebons.dicebons
-- The dice's faces by an RNG2 draw's low four bits (0x022272AC, 0x022272BC).
local FACES_1: { number } = { 1, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6 }
local FACES_2: { number } = { 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 6 }
-- Where a die may land, by side (0x0222728C): solid, nothing on it or reserved; else (0x0222729C) any panel.
local AIM_REQUIRE = { [0] = 0x10, [1] = 0x10 }
local AIM_FORBID = { [0] = 0x1F80_0000, [1] = 0x2F80_0000 }
-- A die: 20 HP, no status or hit modifier, running while dimmed, thrown from 24 pixels above him.
local DIE_HP, DIE_Z = 0x14, 0x18_0000
local THROW_SOUND = asset.sound("minibomb-throw")
local APPEAR_SOUND = asset.sound("appear")
local BONUS_SOUND = asset.sound("hub")

local LEAD_IN, APPEAR, READY, THROW, BONUS, STAND, WARP, LEAD_OUT = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18, 0x1C

type ActorState = { controller: Object?, dice: { boolean }, total: number, navi: Object? }

local function has_flag(chip: Chip, flag: string): boolean
    return chip.flags ~= nil and table.find(chip.flags, flag) ~= nil
end

local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

local function go(me: Object, action: number)
    me.action, me.phase, me.phase_init = action, 0, 0
end

-- 0x021FFC34: where a die lands: of the panels from the far edge back to his column (but `ex`, `ey`), those he may aim
-- at, one drawn with RNG2; none, any panel; and whether it took the second try.
local function aim(me: Object, ex: number, ey: number): (number?, number?, boolean)
    local side = me.alliance
    local front = if bit32.bxor(side, me.flip) == 0 then 1 else -1
    local start = bit32.bxor(bit32.bxor(side, me.flip), 1) * 5 + 1
    local require, forbid = AIM_REQUIRE[side], AIM_FORBID[side]
    local second = false
    while true do
        local found: { number } = {}
        local x = start
        while x ~= me.panel_x do
            for y = 1, 3 do
                if not (x == ex and y == ey) and field.check(x, y, require, forbid) then
                    table.insert(found, x + y * 0x10)
                end
            end
            x -= front
        end
        if #found > 0 then
            local p = found[battle.rng_positive() % #found + 1]
            return bit32.band(p, 7), bit32.rshift(p, 4), second
        end
        if second then
            return nil, nil, true
        end
        second = true
        require, forbid = 0, 0
    end
end

-- 0x021FFD10: a die (its face drawn from `faces`) at (x, y), in slot `slot`; its face.
local function throw_die(me: Object, x: number, y: number, faces: { number }, slot: number): number
    local face = faces[bit32.band(battle.rng_positive(), 0xF) + 1]
    local p = me.pos
    dice.spawn(me, Vec3.new(p.x, p.y, p.z + DIE_Z), {
        face = face,
        target_x = x,
        target_y = y,
        dimmed = true,
        hp = DIE_HP,
        status = nil,
        hit_mod = 0,
        damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
        holder = me,
        slot = slot,
        -- (The DS's division routine, 0x02020818, leaves the hardware divider's address in r3.)
        exact = true,
        -- (0x021FFD44: its +0x6C 1.)
        quiet = true,
    })
    return face
end

-- 0x021FFA51: the throw (animation 6, its sound; 12 ticks); then the dice: the first, and the second (with the side's
-- BattleState +0x44 halfword set, the DS's own: 1 in every recording), each landing where he aimed counting its face.
local function throw(me: Object)
    local s = me.state :: ActorState
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 6
        battle.play_sound(THROW_SOUND)
        me.timer = 0xC
    end
    me.timer -= 1
    if me.timer > 0 then
        return
    end
    s.total = 0
    s.dice[1], s.dice[2] = false, false
    local x, y, second = aim(me, 0, 0)
    if x and y then
        local face = throw_die(me, x, y, FACES_1, 0)
        if not second then
            s.total += face
        end
    end
    local x2, y2, second2 = aim(me, x or 0, y or 0)
    if x2 and y2 then
        local face = throw_die(me, x2, y2, FACES_2, 1)
        if not second2 then
            s.total += face
        end
    end
    go(me, BONUS)
end

-- 0x021FFAE1: once the dice are gone, the bonus: their count times ten; on the navi, the heal's sparkle and the bonus's
-- sound, and the next chip's attack raised by it (on its ninth tick; 30 ticks).
local function bonus(me: Object)
    local s = me.state :: ActorState
    if me.phase_init == 0 then
        if s.dice[1] or s.dice[2] then
            return
        end
        s.total *= 10
        local navi = if s.total ~= 0 then battle.player(me.alliance) else nil
        s.navi = navi
        if navi == nil then
            go(me, STAND)
            return
        end
        me.anim = 0x10
        me.timer = 0x1E
        me.phase_init = 4
    end
    if me.timer == 0x16 then
        local navi = s.navi :: Object
        local cx, cy = field.center(navi.panel_x, navi.panel_y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), effects.heal)
        battle.play_sound(BONUS_SOUND)
        local side = me.alliance
        battle.add_hand_attack_bonus(side, battle.hand_cursor(side), s.total)
    end
    me.timer -= 1
    if me.timer <= 0 then
        go(me, STAND)
    end
end

local function update(me: Object)
    local a = me.action
    if a == LEAD_IN then
        if countdown(me, 4) then
            go(me, APPEAR)
        end
    elseif a == APPEAR then
        if me.phase_init == 0 then
            me.visible = true
            me.anim = 3
            battle.play_sound(APPEAR_SOUND)
        end
        if countdown(me, 4) then
            go(me, if battle.over() then WARP else READY)
        end
    elseif a == READY then
        if me.phase_init == 0 then
            me.anim = 0
        end
        if countdown(me, 0x10) then
            go(me, THROW)
        end
    elseif a == THROW then
        throw(me)
    elseif a == BONUS then
        bonus(me)
    elseif a == STAND then
        if me.phase_init == 0 then
            me.anim = 0
        end
        if countdown(me, 0x14) then
            go(me, WARP)
        end
    elseif a == WARP then
        if me.phase == 0 then
            me.anim, me.anim_loaded = 4, 0
            me.timer = 4
            me.phase = 4
        end
        me.timer -= 1
        if me.timer <= 0 then
            me.visible = false
            go(me, LEAD_OUT)
        end
    elseif a == LEAD_OUT then
        if countdown(me, 4) then
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("DiceBons' NumberMan: action %#x reads past its table (0x0222726C)", a))
    end
    me:update_sprite()
end

dice_bonus.actor = new.kind {
    id = "numberman/dice-bonus",
    pool = "actor",
    state = { controller = "object", dice = "bool[2]", total = "u16", navi = "object" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x021FF948: on the navi's panel, 10 pixels behind it and 10 up, NumberMan's sprite, his shadow.
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x - me.facing * 0xA_0000, p.y - 0xA_0000, 0)
            local sprite = me.sprite
            sprite:load(numberman.sprite)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            -- 0x021FFBF4: his controller's word for him cleared.
            dice_bonus.controller.done((me.state :: ActorState).controller)
            me:free()
        end
    end,
}

-- His controller (the DS's effect object 0x95): the dim, DiceBons' telop, NumberMan (0x022264A8), the undim.
dice_bonus.controller = party_support.controller {
    id = "numberman/dice-bonus-controller",
    bring = function(me: Object): Object?
        local navi = me.related1 :: Object
        local o = battle.spawn(dice_bonus.actor)
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            o.alliance, o.flip = navi.alliance, navi.flip
            o.element = me.element
            o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            o.related1 = navi;
            (o.state :: ActorState).controller = me
        end
        return o
    end,
}

-- 0x02004CC8: whether the dice roll.
local function rolls(side: number): boolean
    if not party.has_partners(side) or not party.light(side) or battle.over() then
        return false
    end
    if bit32.band(bit32.rshift(battle.rng(), 10), 0xF) > CHANCE then
        return false
    end
    if party.dice_bonus_spent(side) then
        return false
    end
    if not battle.link() and side ~= 0 then
        return false
    end
    if not party.may_support(side, numberman) or not party.benched(side, numberman) then
        return false
    end
    if battle.navi(side).navi == numberman then
        return false
    end
    -- The hand's cursor is past the chip just used.
    local cursor = battle.hand_cursor(side)
    if cursor == 0 then
        return false
    end
    local used = battle.hand_chip(side, cursor - 1)
    if used == nil or has_flag(used, "dimming") then
        return false
    end
    local next = battle.hand_chip(side, cursor)
    if next == nil or not has_flag(next, "has_damage") then
        return false
    end
    party.spend_dice_bonus(side)
    return true
end

-- The rules' `chip_ended`: the dice roll; NumberMan comes (0x022264DC, no damage of his own).
function dice_bonus.chip_ended(side: number, navi: Object)
    if rolls(side) then
        dice_bonus.controller.send(navi, 0, DICEBONS)
    end
end

return dice_bonus
