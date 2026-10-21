--!strict
-- MegaMan's party support (EXE5 DS: 0x02004130, his actor the support actor's variant 5, 00-00;
-- docs/design/exe5ds-map.md §4.7): with NaviCust's Spport (or in the unlimited mode), while a team navi fights in his
-- place, every 130 to 250 ticks outside the dimming he comes and attacks: by RNG2 (weights 6/6/4, of those that find a
-- panel) his buster from before the farthest enemy, his sword before the nearest, or a MiniBomb thrown from three panels
-- before an enemy. He goes back as soon as the battle is over or he is on the field again; his timer starts again as he
-- goes (or when he can't come).

local attachment = require("../../objects/attachment")
local base_form = require("./forms/base").base
local bombs = require("../../lib/bombs")
local collision = require("../../rules/collision")
local exe5 = require("../../rules/api")
local hub_shade = require("./forms/hub_shade")
local megaman = require("../megaman").megaman
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local projectile = require("../../objects/projectile")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local supports = require("../../rules/party/supports")

-- His timer (0x020041E4: RNG2 bits 6–9 into 0x020CC2E8's offsets of 0x020CC2E0's ticks, weights 2/6/6/2).
local TIMER: { number } = { 130, 130, 170, 170, 170, 170, 170, 170, 210, 210, 210, 210, 210, 210, 250, 250 }
-- His attacks' damage by the side's level (hit flags 0x9E): the buster's (0x020CC2F8), the sword's (0x020CC30C), the
-- bomb's (0x020CC320).
local BUSTER_DAMAGE: { number } = { 20, 40, 60, 60, 80, 80, 100, 110, 120, 120 }
local SWORD_DAMAGE: { number } = { 25, 50, 70, 70, 90, 90, 110, 140, 150, 150 }
local BOMB_DAMAGE: { number } = { 30, 60, 80, 80, 100, 100, 120, 170, 180, 180 }
local FLAGS = 0x9E
-- Where he may stand, by his side (solid and his side's own, nothing on it or reserved): the buster before the
-- farthest enemy, 5 to 1 columns in front of it (Colonel's, 0x020CC27C); the sword the panel before the nearest
-- (0x020CC354); the bomb three panels before any (0x020CC370, one drawn).
local REQUIRE = { [0] = 0x10, [1] = 0x30 }
local FORBID = { [0] = 0x0F8800A0, [1] = 0x0F880080 }
local BUSTER_AT: { { { number } } } = { { { 5, 0, 0 } }, { { 4, 0, 0 } }, { { 3, 0, 0 } }, { { 2, 0, 0 } }, { { 1, 0, 0 } } }
local SWORD_AT: { { { number } } } = { { { 1, 0, 0 } }, { { 2, 0, 0 } } }
local BOMB_AT: { { { number } } } = { { { 3, 0, 0 } } }
local BUSTER, SWORD, BOMB = 0, 1, 2
-- The weights of each found (6, 6, 4), drawn from RNG2 bits 7 on.
local WEIGHTS = { [BUSTER] = 6, [SWORD] = 6, [BOMB] = 4 }

-- His buster: animation 8, the cannon held (the attachment table's row 0); on its 16th tick the cannon's sound and a
-- shot from the panel ahead 24 pixels up (the DS's projectile row 0x1F: the cannon's, no hit modifier); 29 ticks; then
-- animation 7, 4 ticks.
local CANNON = attachment.look { sprite = asset.sprite("cannon"), palette = 0, attach_point = 7 }
local CANNON_SOUND = asset.sound("cannon")
local SHOT = projectile.variant { id = "shot/megaman-support", collision = { is = collision.attack, hits = collision.hits_navis }, hit_mod = 0, element = "null", spark = sparks.impact }
-- His sword: animation 5, the sword (row 3), its sound; on its 10th tick a hit over the panel ahead and the one past it
-- (region 2, a slash at navis, on the ground) and the long slash 16 pixels up (effect 0x17); 22 ticks; then 6.
local SWORD_LOOK = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SWING_SOUND = asset.sound("sword-swing")
local LONG_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 0 }
-- His bomb: animation 6, the bomb held (row 4), the throw's sound; on its 10th tick MiniBomb's bomb (the DS's bomb row 0)
-- from 4 pixels before him, 48 up; 21 ticks; then 6.
local BOMB_LOOK = attachment.look { sprite = asset.sprite("bomb"), palette = 0 }
local THROW_SOUND = asset.sound("minibomb-throw")
local THROWER = bombs.thrower(bombs.variant {
    collision = { is = collision.thrown, hits = collision.hits_navis },
    hit_mod = 0,
    palette = 0,
    anim = 1,
    blast = regions.single,
})

type State = { member: number, attack: number }

local support = {}

-- 0x020041E4: his timer starts again.
local function reset(side: number)
    party.supports(side).megaman_timer = supports.pick(TIMER, 6, true)
end

-- His member block (benched, his stats are there).
local function block(me: Object): Navi?
    local m = exe5.member_of(me.alliance, megaman)
    return if m then battle.member(me.alliance, m) else nil
end

-- 0x021FF3E4: his palette by his block: Hub Style's (0x19 for style 1, 0x1E for 2), plus 1 when his mood is 0, else his
-- light/dark value's tier.
local function palette(me: Object): number
    local b = block(me)
    if b == nil then
        return 0
    end
    local p = if b.hub_style == 1 then 0x19 elseif b.hub_style == 2 then 0x1E else 0
    if b.mood == 0 then
        return p + 1
    end
    return p + exe5.light_dark_tier(party.megaman_karma(me.alliance))
end

-- 0x021FF434: in Hub Style (but the DS's style 3) Hub Style's shade.
local function wears(me: Object)
    local b = block(me)
    if b and b.hub_style ~= 0 and b.hub_style ~= 3 then
        me:add_parts(hub_shade, 0)
    end
end

-- A count down that ends once the timer falls below 0 (`bge`).
local function fell_below(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    return t < 0
end

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- 0x021FEED1, 0x021FEF2D: the buster.
local function buster(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.anim = 8
            me.timer = 0
            attachment.attach(me, CANNON, "related", { anim = 0, while_dimmed = true })
            me.phase_init = 4
            return
        end
        if me.timer == 0xF then
            battle.play_sound(CANNON_SOUND)
            projectile.spawn(me, me.panel_x + me.facing, me.panel_y, { variant = SHOT, damage = damage_word(me), z = 0x18_0000 })
        end
        me.timer += 1
        if me.timer == 0x1D then
            me.phase, me.phase_init = 4, 0
        end
    else
        if me.phase_init == 0 then
            me.related1 = nil
            me.anim = 7
        end
        if fell_below(me, 3) then
            party_support.go(me, party_support.WARP)
        end
    end
end

-- 0x021FEF69, 0x021FEFE9: the sword.
local function sword(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.anim = 5
            battle.play_sound(SWING_SOUND)
            attachment.attach(me, SWORD_LOOK, "related", { anim = 0, while_dimmed = true })
            me.timer = 0x15
            me.phase_init = 4
        end
        if me.timer == 0xC then
            local x, y = me.panel_x + me.facing, me.panel_y
            battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = me.element,
                z = 0,
                region = regions.long,
                target = collision.hits_navis,
                self_type = collision.slash,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 0,
            })
            local cx, cy = field.center(x, y)
            battle.effect(Vec3.new(cx, cy, 0x10_0000), LONG_SLASH, bit32.bxor(me.alliance, me.flip))
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 4, 0
        end
    elseif fell_below(me, 5) then
        me.related1 = nil
        party_support.go(me, party_support.WARP)
    end
end

-- 0x021FF021, 0x021FF091: the bomb.
local function bomb(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.anim = 6
            attachment.attach(me, BOMB_LOOK, "related", { anim = 0, while_dimmed = true })
            battle.play_sound(THROW_SOUND)
            me.timer = 0
            me.phase_init = 4
        end
        if me.timer == 9 then
            local p = me.pos
            THROWER.throw(me, Vec3.new(p.x + me.facing * 0x4_0000, p.y, p.z + 0x30_0000), damage_word(me))
            me.related1 = nil
        end
        me.timer += 1
        if me.timer >= 0x15 then
            me.phase, me.phase_init = 4, 0
        end
    elseif fell_below(me, 5) then
        party_support.go(me, party_support.WARP)
    end
end

local ATTACKS = { [BUSTER] = buster, [SWORD] = sword, [BOMB] = bomb }

support.actor = party_support.make {
    id = "megaman/support",
    navi = megaman,
    sprite = base_form.sprite,
    palette = palette,
    wears = wears,
    warp_keeps_parts = true,
    appear_ticks = 0x14,
    state = { attack = "u8" },
    actions = {
        -- 0x021FEE91: back as the battle ends or he is on the field again; else his attack.
        [party_support.ATTACK] = function(me: Object)
            if battle.over() or battle.navi(me.alliance).navi == megaman then
                party_support.go(me, party_support.WARP)
                return
            end
            ATTACKS[(me.state :: State).attack](me)
        end,
    },
    gone = function(me: Object)
        reset(me.alliance)
    end,
}

-- 0x0200420C: his attack: each that finds a panel (each search drawing), weighted, one drawn.
local function choose(navi: Object): (number?, number?, number, number)
    local side = navi.alliance
    local found: { { number } } = {}
    local bx, by, bflip = supports.target(navi, BUSTER_AT, REQUIRE[side], FORBID[side], true)
    if bx and by then
        local e: { number } = { BUSTER, bx :: number, by :: number, bflip :: number }
        table.insert(found, e)
    end
    local sx, sy, sflip = supports.target(navi, SWORD_AT, REQUIRE[side], FORBID[side])
    if sx and sy then
        local e: { number } = { SWORD, sx :: number, sy :: number, sflip :: number }
        table.insert(found, e)
    end
    local spots: { { number } } = {}
    for i = 1, 4 do
        local enemy = battle.alive_actor_slot(1 - side, i)
        if enemy then
            local x, y, flip = supports.around(enemy, BOMB_AT, REQUIRE[side], FORBID[side])
            if x and y then
                local e: { number } = { x :: number, y :: number, flip :: number }
                table.insert(spots, e)
            end
        end
    end
    if #spots > 0 then
        local p = spots[battle.rng_positive() % #spots + 1]
        table.insert(found, { BOMB, p[1], p[2], p[3] })
    end
    local weights: { number } = {}
    for i, f in found do
        for _ = 1, WEIGHTS[f[1]] do
            table.insert(weights, i)
        end
    end
    if #weights == 0 then
        return nil, nil, 0, 0
    end
    local f = found[weights[bit32.rshift(battle.rng_positive(), 7) % #weights + 1]]
    return f[2], f[3], f[4], f[1]
end

local DAMAGES = { [BUSTER] = BUSTER_DAMAGE, [SWORD] = SWORD_DAMAGE, [BOMB] = BOMB_DAMAGE }

-- 0x02004130 (each fight tick): the light test, MegaMan a benched member who may support; his timer set once a turn;
-- outside the dimming, his timer runs out: no member busy, the side's navi a team navi, an attack; else his timer
-- starts again (but with MegaMan on the field).
function support.tick(side: number)
    if not party.light(side) then
        return
    end
    local m = party.member_of(side, megaman)
    if m == nil or not party.may_support(side, megaman) or not party.benched(side, megaman) then
        return
    end
    local s = party.supports(side)
    if not s.megaman_set then
        s.megaman_set = true
        reset(side)
    end
    if battle.dimmed() then
        return
    end
    s.megaman_timer = bit32.band(s.megaman_timer - 1, 0xFFFF)
    if s.megaman_timer ~= 0 then
        return
    end
    local navi = battle.player(side)
    if navi == nil then
        return
    end
    if party.any_busy(side) then
        reset(side)
        return
    end
    if battle.navi(side).navi == megaman then
        return
    end
    local x, y, flip, attack = choose(navi)
    if x == nil or y == nil then
        reset(side)
        return
    end
    local o = party_support.send(support.actor, navi, x, y, flip, supports.damage(side, DAMAGES[attack], FLAGS), m)
    if o then
        (o.state :: State).attack = attack
    end
end

return support
