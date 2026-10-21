--!strict
-- KnightMan's guard (EXE5 DS's party battle: the hit intake's 0x02063B72 and its gate 0x02004604, the navi's action 0x5B,
-- 0x02049784, his actor the support actor's variant 6, 08-0B, 0x021FF128, the damage he takes 0x020046D4;
-- docs/design/exe5ds-map.md §4.7): when a hit no trap caught reaches the navi standing at its area's front edge, with
-- the light test passed and KnightMan a member who may support (not on the field) above a quarter of his HP, the hit is
-- swallowed and the navi stands still for 61 ticks with KnightMan before it; every hit swallowed meanwhile flashes him
-- white with a sound, and its damage comes off his HP instead (to 1 at least). Not once a battle.

local knightman = require("../knightman").knightman
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")

local guard = {}

-- What the guard reads as the side of a panel off the field: the DS's `object_getPanelDataOffset` (0x0208C320) gives
-- none for one, and the guard reads its record's side unchecked (0x02049794): the byte at address 3, the ARM9's ITCM
-- (mirrored from 0), whose fourth byte is 0xE9 (its first instruction, a `stmdb`). Never a side.
local OFF_FIELD_SIDE = 0xE9
-- The guard's ticks; the swallowed hit's sound on KnightMan.
local GUARD_TICKS = 0x3C
local FLASH_SOUND = asset.sound("own-hit")

type ActorState = { member: number, flash: boolean, kept: boolean }
type ActionState = { timer: number }

-- 0x021FF1C5: he stands; a hit swallowed (0x021FF3D8's flag) shows white for a tick with its sound; once the navi is no
-- longer guarded, he warps out.
local function stand(me: Object)
    local s = me.state :: ActorState
    me.sprite.white = false
    if s.flash then
        s.flash = false
        me.sprite.white = true
        battle.play_sound(FLASH_SOUND)
    end
    local navi = me.related1
    if navi == nil or not navi:navi_state("guarded") then
        party_support.go(me, party_support.WARP)
    end
end

guard.actor = party_support.make {
    id = "knightman/guard",
    navi = knightman,
    appear_ticks = 4,
    -- (0x021FF18A: a pixel down the field and a pixel up; his 20 pixels forward land in the fraction's halfword alone,
    -- which they leave as it was.)
    appearing = function(me: Object)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y + 0x1_0000, p.z + 0x1_0000)
    end,
    actions = { [party_support.ATTACK] = stand },
    -- (0x021FE5D0: the party block's slot for him cleared, while he is the one it keeps.)
    released = function(me: Object)
        if (me.state :: ActorState).kept then
            party.supports(me.alliance).knightman_guard = nil
        end
    end,
    state = { flash = "bool", kept = "bool" },
}

-- 0x02004604: the navi stands at its area's front edge (the panel ahead on the field and the other side's), the light
-- test, KnightMan a member who may support, above a quarter of his maximum HP and not on the field.
local function ready(side: number, navi: Object): boolean
    if not party.light(side) then
        return false
    end
    local m = party.member_of(side, knightman)
    if m == nil then
        return false
    end
    local ahead = field.panel(navi.panel_x + navi.facing, navi.panel_y)
    if ahead == nil or ahead.alliance == side then
        return false
    end
    if not party.may_support(side, knightman) then
        return false
    end
    local stats = battle.member(side, m) :: Navi
    if stats.hp <= bit32.rshift(stats.max_hp, 2) then
        return false
    end
    return battle.navi(side).navi ~= knightman
end

-- 0x0200467C: KnightMan before the navi (on its panel, its member busy, facing as it does), the party block's guard
-- (the one it kept before told to go).
local function bring(side: number, navi: Object)
    local m = party.member_of(side, knightman) :: number
    local o = party_support.send(guard.actor, navi, navi.panel_x, navi.panel_y, navi.flip, 0, m)
    if o == nil then
        return
    end
    local s = party.supports(side)
    local old = s.knightman_guard
    if old then
        old:set_lifecycle("destroy");
        (old.state :: ActorState).kept = false
    end
    s.knightman_guard = o;
    (o.state :: ActorState).kept = true
end

-- 0x020497DA: the guard ends (the navi's guarded state with it, 0x020497E2).
local function finish(me: Object)
    me.related1 = nil
    me.overlay = nil
    me:set_navi_state("guarded", false)
    me:exit_attack()
end

-- 0x02049784 (the roles' `guard`): while the navi stands at its area's front edge (the panel ahead not its side's; one
-- off the field never is, `OFF_FIELD_SIDE`) and isn't diving, 61 ticks of the guard (the navi guarded, KnightMan
-- brought on the first, the navi's animation 0).
local function update(me: Object, s: ActionState)
    local ahead = field.panel(me.panel_x + me.facing, me.panel_y)
    local side = if ahead then ahead.alliance else OFF_FIELD_SIDE
    if side == me.alliance or me:navi_state("diving") then
        me:set_request("guard", false)
        return finish(me)
    end
    if me.step_init == 0 then
        me:set_navi_state("guarded", true)
        me:set_request("guard", false)
        bring(me.alliance, me)
        me:set_animation(0)
        s.timer = GUARD_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        finish(me)
    end
end

guard.action = new.action { id = "knightman/guard/action", state = { timer = "u16" }, update = update }

-- 0x02063B72 (the rules' `hit_guard`): a hit no trap caught is swallowed while the navi is guarded, or asks for the
-- guard; it then needs the navi ready for it (`ready`), a hit at all (the hit flags' 0xFF800000) and damage in it.
function guard.hit_guard(side: number, navi: Object): boolean
    if navi:navi_state("guarded") then
        return true
    end
    if navi:navi_state("diving") then
        return false
    end
    if navi:request("guard") then
        return true
    end
    if not ready(side, navi) then
        return false
    end
    local c = navi.collision
    if bit32.band(c.hit_flags, 0xFF800000) == 0 then
        return false
    end
    local any = 0
    for e = 0, 4 do
        any = bit32.bor(any, c:element_damage(e))
    end
    if any == 0 then
        return false
    end
    navi:set_request("guard", true)
    return true
end

-- 0x02063BC2 and 0x020046D4 (the rules' `hit_swallowed`): every hit a trap or the guard swallows adds its damage (half,
-- rounded up, on a holy panel) to what KnightMan's guard has yet to take; with his guard out it flashes, and while
-- KnightMan may support it comes off his HP (to 1 at least).
function guard.hit_swallowed(side: number, navi: Object)
    local c = navi.collision
    local sum = 0
    for e = 0, 4 do
        sum += c:element_damage(e)
    end
    local here = field.panel(navi.panel_x, navi.panel_y)
    if here and here.kind == "holy" then
        sum = bit32.rshift(sum + 1, 1)
    end
    local s = party.supports(side)
    local pending = bit32.band(s.knightman_pending + sum, 0xFFFF)
    s.knightman_pending = pending
    local k = s.knightman_guard
    if pending == 0 or k == nil then
        return
    end
    (k.state :: ActorState).flash = true
    if not party.may_support(side, knightman) then
        return
    end
    local stats = battle.member(side, party.member_of(side, knightman) :: number) :: Navi
    local hp = stats.hp
    stats.hp = hp - (if hp > pending then pending else hp - 1)
    s.knightman_pending = 0
end

return guard
