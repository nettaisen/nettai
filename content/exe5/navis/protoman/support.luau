--!strict
-- ProtoMan's party support (EXE5 DS: 0x020037A8, his actor the support actor's variant 0, 08-01; docs/design/exe5ds-map.md
-- §4.7): once a turn, on the fight's second tick outside the dimming, he comes before the enemy nearest the navi on the
-- field (else above or below it) and slashes the panel ahead.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local protoman = require("../protoman").protoman
local regions = require("@exelib/regions")
local supports = require("../../rules/party/supports")

-- His damage by the side's level (0x020CC1CC), its hit flags 0x94.
local DAMAGE: { number } = { 20, 50, 70, 70, 90, 90, 110, 140, 150, 150 }
local FLAGS = 0x94
-- Where he may stand (0x020CC1E0): in front of the enemy, else above or below that; solid (0x10), nothing on it or
-- reserved, of either side (0x0F880080).
local OFFSETS: { { { number } } } = { { { 1, 0, 0 } }, { { 1, -1, 0 }, { 1, 1, 0 } } }
local REQUIRE, FORBID = 0x10, 0x0F880080

-- His swing (animation 5, his sword: the attachment table's row 3 in its animation 1, the swing's sound), its hit on
-- its 11th tick, 16 pixels up (the panel ahead and the two beside it, as a slash at navis), the slash shown there (the
-- effect table's row 0x33); 51 ticks.
local ANIM_SWING = 5
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SWORD_ANIM = 1
local SWING_SOUND = asset.sound("sword-swing")
local SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
local HIT_AT, SWING_ENDS, SWING_Z = 0xA, 0x32, 0x10_0000

-- 0x021FE66D: the swing.
local function swing(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_SWING
        attachment.attach(me, SWORD, "related", { anim = SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SWING_SOUND)
        me.timer = 0
        me.phase_init = 4
        return
    end
    if me.timer == HIT_AT then
        local x, y = me.panel_x + me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = SWING_Z,
            region = regions.wide,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 0,
        })
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, SWING_Z), SLASH, bit32.bxor(me.alliance, me.flip))
    end
    me.timer += 1
    if me.timer > SWING_ENDS then
        party_support.go(me, party_support.WARP)
    end
end

local support = {}

support.actor = party_support.make {
    id = "protoman/support",
    navi = protoman,
    appear_ticks = 0xC,
    actions = { [party_support.ATTACK] = swing },
}

-- 0x020037A8 (each fight tick): once a turn (the party block's +0 and its counter +1, from 0x02003B98), on the second
-- tick outside the dimming; the light test, ProtoMan a member who may support (spent here, whatever comes next), the
-- side's navi not ProtoMan, no member busy, a panel by an enemy.
function support.tick(side: number)
    local s = party.supports(side)
    if s.protoman_used or battle.dimmed() then
        return
    end
    s.protoman_count = bit32.band(s.protoman_count + 1, 0xFF)
    if s.protoman_count ~= 2 or not party.light(side) then
        return
    end
    local m = party.member_of(side, protoman)
    if m == nil or not party.may_support(side, protoman) then
        return
    end
    s.protoman_used = true
    local navi = battle.player(side)
    if navi == nil or battle.navi(side).navi == protoman or party.any_busy(side) then
        return
    end
    local x, y, flip = supports.target(navi, OFFSETS, REQUIRE, FORBID)
    if x and y then
        party_support.send(support.actor, navi, x, y, flip, supports.damage(side, DAMAGE, FLAGS), m)
    end
end

return support
