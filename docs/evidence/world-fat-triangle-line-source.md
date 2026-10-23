# World fat-triangle line continuation

2026-09-08, original debug XEX/PDB read-only. This is the concrete
fifth wheel/deck batch query dependency. Confidence **exact** for source symbols,
raw gates, iteration/result ordering and wrapper; **high** for the named vector
graph below. This packet does not certify a completed implementation or a Xenon
reference trace. No Cargo or geometry implementation was performed.

## Scope and existing mismatch

The deck test requires line fatness word `3e19999a`, along the actual part6
position to position+[0,-100,0,0]. Existing `collision_triangle_line.rs`
explicitly refuses nonzero sum fatness; that refusal must not become a miss.

`skate2-formats/src/collision.rs::sphere_segment_triangle_intersection` already
provides a useful geometric diagnostic, but is **not** an equivalent native
implementation. It rejects negative radius up front, enumerates face/edge/vertex
candidates independently and chooses the earliest, and uses a different nearest
point algorithm. Native uses plane/barycentric early rejects and a bounded
nearest-region walk, with partial result writes even on misses and a successful
iteration-limit exit. Do not wire the generic helper into World and call it
source parity. Its descriptive source comment is weaker than the actual raw
contract established here; this audit leaves that shared file unchanged.

## Source symbols, types, callers

| Function | Half-open range | SHA256 |
|---|---|---|
| rw::collision::FatTriangleLineSegIntersect | 82adeb48..82adf654 | b4a9ac1cead69c63a0193908502055e420e8cb0bce40e4b9c22701c5f3a6ea31 |
| rw::collision::TriangleLineSegIntersect | 82adf658..82adf804 | 4a0732cc1624cd8f8517f88772c64498e9fee495bd77d5dd047908f8526016ce |
| rw::collision::rwcSphereLineSegIntersect | 82acdef0..82acdfe4 | 9594605882c5cdaddebcaef516fb1d6a19e961a9ed69e74dd3a6af1be4d9ce34 |
| rw::collision::rwc_TriangleNearestPoint | 82ad9a40..82ad9cf4 | 5dbaecd9b2415a2d2f641aa90d55ec57afc14dd07915cc020be86c068ec61d84 |

PDB `VolumeLineSegIntersectResult` size d0: volume pointer+0, position+10,
normal+20, volParam+30, lineParam+40, volume reference+50. Fat helper parameters
are result,start,delta,v0,v1,v2,radius; decompiler's extra floating arguments
are allocation artifacts. It explicitly reports failed local-variable allocation.
Read raw before trusting its repeated temporary names, overwritten fractions,
or region expressions.

PDB Fraction is two floats `{num+0,den+4}`. PDB region values are vertex0=0,
vertex1=1, vertex2=2, edge0=3 (v0→v1), edge1=4 (v0→v2), edge2=5 (v1→v2),
face=6. The helper is called by TriangleLineSegIntersect at82adf7b0; the
existing clustered mesh line traversal reaches that wrapper for each actual GP
triangle. Mesh passes triangle fatness+0 and the original request line fatness.

## Wrapper: radius selection and visible prefix

At82adf668 capture original lineFatness in f31. `fadds f1,f31,f2` atf684 gives
sumRadius. `fcmpu` then branchf68c selects fat when not ordered equal to zero,
including NaN. Either signed zero sum selects the existing thin branch. There
is **no negative-radius or finite-radius validation** in this wrapper or entry
to the fat helper; do not add `radius<0 => miss` or `is_finite` sanitization.
Negative, infinity and NaN pass through the actual arithmetic/branches. A
negative line radius plus positive triangle radius can still produce zero sum.

For fat branch, compute raw cross `(v0-v1)×(v0-v2)` with source VMX permutation
0x63 and fused negative multiply-subtract. Publish raw normal before dot/rsqrt,
then normalize using dot3, reciprocal-square-root estimate and TWO source
refinements, and publish normalized normal (`f724..f7ac`). This normal is an
input to the fat helper, not an optional postprocessing value. A numeric failure
therefore retains the raw normal and does not clear lineParam yet.

Fat entry first writes lineParam+0 at**82adeb7c**. On false, wrapper returns
without position adjustment. On true, it reads resulting normal and position
and subtracts **normal * original lineFatness**, separate full4 multiply/subtract
at**82adf7ec/f7f0**, then stores positionf7f4. It uses neither sumRadius nor
triangleFatness for this adjustment. The inner position is sphere-center
position; the wrapper position is offset toward the contacted surface.

Keep volume pointer and volume-reference fields untouched. Existing
`CollisionTriangleLineOutput` Option fields can retain this exact write/readiness
graph; never reconstruct an all-zero complete result after a partial source path.

## Initial plane and barycentric rejection

Notation: original start S, delta D, vertices A/B/C, edges E1=B-A, E2=C-A,
offset O=S-A, incoming normalized triangle normal N. Use full4 vector operations
and numeric dot3; only stated scalar reductions ignore W.

1. Form P=D×E2, determinant d=dot(E1,P). Sign starts+1 and changes to-1 only
   for ordered d<0 (ebf4 raw40980010 skips sign change for unordered). Make
   positive determinant by negation only in that branch, not generic abs.
2. Reject only ordered `sign*dot(N,O) < -radius` at**ec28**. There is no
   position or volParam store on this rejection; initial lineParam0 and wrapper
   normal prefix remain.
3. Clear sign bits of P lanes, reduce absolute components as **(Y+Z)+X**,
   then multiply by radius for barycentric slack. U=sign*dot(O,P).
   Reject ordered U < -slack or U > slack+d (**ec78/ec84**).
4. Q=E1×D; similarly compute V and its separate slack with the same component
   reduction. Reject ordered V < -slackV, V > slackV+d, or
   **(V+U) > (slackU+slackV)+d** (**ece4/ecf0/ed04**). Raw separate additions
   fix the parentheses; decompiler incorrectly repeats its current temporary.
5. The next gate is determinant ordered greater than the original
   **MINIMUM_RECIPROCAL word00200000** (`ed14` raw40990168 is GT-not-set).
   Zero, small and NaN determinant skip face entry toward nearest-region work.

These early rejection branches are LT-set/GT-set, so NaNs are not broadly
rejected. Do not rewrite all range gates with Rust range contains.

## Face entry and segment reduction

For determinant above threshold, shift O by `N*(sign*radius)` to test the
offset face. Compute the plane numerator from the edge cross and sign
(`ed18..ed64`); reject only ordered numerator>d (`ed6c`), and skip face work
to nearest-region when numerator<0 (`ed74`). Otherwise first capture the
shifted-U dot reduction at **ed78**. Only then compute reciprocal
`1/d` via scalar **fdivs**, then parameter=reciprocal*numerator and publish
lineParam at**ed88**, before barycentric acceptance. Multiply the captured U
by sign at **ed94**, after publication. A fallible host reduction at ed78 must
retain entry lineParam+0; the later shifted-V reduction retains the face t.

Shifted U rejection is ordered U<0 or U>d (`eda0/eda8`); shifted V rejects
ordered V<0 or V+U>d (`edc8/edd4`). Unordered comparisons fall through.
Accepted face writes position=S+D*t using full4 FMA, then normal=N*sign,
then volParam=[U/d,V/d,+0,+0], and returns true (**ee24..ee44**).

If face parameter was committed but barycentric region rejected, reduce the
remaining segment: offset becomes O+D*t, delta becomes D*(1-t), using FMA then
separate multiply. The retained result lineParam already records advancement.
No generic re-evaluation from the original segment may erase that prefix.

## Nearest-region entry and early hits

Call `rwc_TriangleNearestPoint` at**eea8** with current world start after
reduction, original vertices, and separate writable point/U/V outputs. Its
PDB region enum is required; the generic formats helper does not preserve
native region ties or degenerate-triangle classification.

Compute offset=currentStart-nearestPoint and squared length. The overlap test
uses **fmsubs radius*radius - squared** at**eed0**, not multiply then subtract.
Only ordered positive enters overlap (`eed8` is GT-not-set skip, includesNaN).
Overlap writes center position first (**ef0c**), computes offset-normal using
one rsqrt refinement, selecting the existing normal if squared is not ordered
positive, then writes normal and volParam=[U,V,penetrationSquared,+0]. The
positive volParam.Z is the squared overlap expression, not penetration depth.
It returns true with the current already-retained lineParam.

Without overlap, region FACE returns true: position first, optional normal
flip only if retained f2 is ordered negative, then volParam=[U,V,0,0]. f2 is
the sign-adjusted determinant: ebec loads it, ec00 negates an ordered negative
value, and neither intervening instructions nor the nearest helper write f2.
Thus ef74/ef7c cannot flip a finite determinant; NaN also skips the flip. The
original orientation sign lives separately at stack50 and must not drive this
gate. Other regions enter the bounded transition loop.

## Region walk: fractions, tie order, and the five-step limit

Counter initialized5 at**ebc8**; decrement at each loop entry**efe4**. Fractions
remain numerator/denominator until the actual selected advance; this avoids
substituting rounded divisions into candidate comparisons.

For vertex region0/1/2, call sphere helper**f000** centered at its nearest
vertex. Negative return immediately rejects. The helper's zero result can still
be replaced by a boundary transition. Construct outgoing vectors to the other
two vertices in source order. For each: require ordered dot(delta,edge) >
threshold and ordered dot(vertex-currentStart,edge)>0. Select boundary fraction
when no current candidate OR comparison is NOT ordered less:
`denNew*numOld < numNew*denOld` skips replacement; equal and unordered replace.
First transition sets region trunc((oldVertex+6)/2), second uses
trunc((oldVertex+9)/2). Thus source iteration order determines ties. If selected
kind remains sphere-hit (<=2), take vertex-hit exit. Otherwise update region
and advance the remaining segment.

For edge region3/4/5, choose origin/direction A/B, A/C, or B/C respectively.
Compute squared cross of (edgeOrigin-currentStart) with edge, and threshold
**(edgeLengthSquared*radius)*radius**, preserving two multiplies. Ordered
inside-cylinder chooses fraction0/1 (**f1dc..f1e4**). Otherwise construct the
edge×delta projection and require ordered positive approach (**f210**, NaN
rejects). The quadratic discriminant graph is:

`product = a * crossOffsetSquared; inner = fmsubs(b,b,product);`
`discriminant = fmadds(a, radiusTerm, inner)` (**f224..f22c**).

The gate atf234 is LT-not-set, so NaN reaches the following boundary checks;
these are ordered LT-set tests. Accept the cylinder candidate only if
`b-a < 0` OR `(b-a)^2 < discriminant`. On acceptance, scalar sqrt atf260,
store denominator first then numerator `b-sqrt(discriminant)`.

Then test the two edge endpoint transition planes in native order with the
same fractional comparison/tie behavior. Positive direction transitions to
vertex trunc(edgeRegion/2); negative direction transitions to
trunc((edgeRegion-3)/2). These stores must copy the actual float num/den pair:
decompiler's `__PAIR64__(&v129,...)` is an allocation error, not pointer data.
If an edge region remains selected, take edge-hit exit; a vertex selection
continues the bounded loop.

Shared advance (**f36c..f3d4**): scalar t=num/den; read old result t0; compute
new parameter with **FMA((1-t0),t,t0)** and STORE atf3b0; then full4 FMA current
start+=delta*t, scale delta by1-t, and store both scratch vectors. Only AFTER
these mutations does ordered t>1 reject (**f3c8**). If counter is now0 it
returns TRUE atf62c without a final position/normal/volParam publication.
Do not change this source limit to an unbounded search, failure, or fabricated
complete hit. Host safe readiness handling must report a genuine missing
source output if this path reaches an uninitialized consumer.

Vertex-hit exit (**f3dc..f4c4**): divide fraction, prepare indicator barycentrics
[vertex==1,vertex==2,0,0], obtain radius reciprocal estimate with TWO refinements,
advance result parameter by the same FMA, publish center position, publish raw
center-minus-vertex normal BEFORE refined reciprocal multiplication, then
publish scaled normal and volParam. No general safe normalization replaces
division by radius.

Edge-hit exit (**f4cc..f638**): divide fraction and edge length squared; advance
parameter, compute center by FMA, project onto edge and form center-minus-edge
point. Publish raw normal then normal/radius, then center position. Finally
publish [edgeFraction,0,0,0] for edge0, [0,edgeFraction,0,0] for edge1, or
[1-edgeFraction,edgeFraction,0,0] for edge2. The radius inverse uses the same
two estimate refinements; edge squared inverse is scalar fdivs. The order of
normal versus position writes differs from vertex-hit and must remain visible.

## Sphere fraction helper, including exceptional paths

`rwcSphereLineSegIntersect82acdef0` uses C=center-origin, A=dot(delta,delta),
B=dot(C,delta), C2=dot(C,C). It does not special-case negative radius.

* If ordered C2<radius², store num0 then den1 and return1. NaN comparison
  does not enter overlap (`df18` LT-not-set).
* Otherwise require ordered B>0 (`df5c` GT-set), else return **-1**, including
  NaN B. This distinction is consumed by the region walker.
* Discriminant is **fmsubs(A,radius²,|C×delta|²)** atdfa0. Ordered negative
  returns0; unordered passes the LT-not-set gate atdfa8.
* If ordered B-A>0 AND ordered (B-A)²>discriminant, return0. Otherwise sqrt,
  store denominator A atdfd0, then numerator B-sqrt atdfdc and return1.
* A miss/-1 leaves Fraction untouched; positive path output order is not
  universally num-before-den.

This differs from the existing diagnostic `ray_sphere_parameter`, including
its early A<=0 check and immediately divided Option parameter.

## Nearest-point normalized case table

The nearest-point helper computes E1/E2 and vertex0-input offset, dot terms,
determinant and region numerators with original FMS graphs. It has an explicit
degenerate determinant gate against word00200000, selects a longest-edge route
with source tie order, and then publishes outU, outV before full4 point at
**9cc8/9ccc/9cec**. Existing formats nearest-point is a different six-dot
Voronoi enumeration and cannot certify source degeneracy/ties. The raw scalar
register graph resolves as follows (all LT/GT below mean ordered; `not LT`
and `not GT` deliberately include unordered):

Let a=dot(E1,E1), b=dot(E1,E2), c=dot(E2,E2), d=dot(E1,A-point),
e=dot(E2,A-point). Dot sampling order is a,c,d,e,b (9a8c..9ac8).
Compute separate products b*b,d*c,e*a, then
det=FMS(c,a,b*b), u=FMS(e,b,d*c), v=FMS(d,b,e*a) (9ad0..9ae4).
The opposite edge squared length L is `FNMS(b,2,a)+c`, preserving the fused
negative multiply-subtract and later addition (9af0/9af8 or9c0c/9c10).

First choose an internal route (not yet the final enum):

* If det LT word00200000: if a GT c, choose edge0 when a GT L, else edge2;
  otherwise choose edge1 when c GT L, else edge2. Equal lengths prefer the
  later selected edge; NaN det enters the nondegenerate route.
* Otherwise, if (v+u) GT det: when u LT0, choose edge1 only if (e+c) LT(d+b),
  else edge2. When u is not LT0, choose edge0 only if v LT0 AND
  (d+a) LT(e+b), else edge2.
* Otherwise: if u LT0 choose edge1 if -e GT0, else edge0. If u is not LT0
  and v LT0 choose edge0 if -d GT0, else edge1. If neither is LT0, choose face.

Then resolve the selected route:

| Route | Exact resolution |
|---|---|
| face | reciprocal=scalar1/det; U=reciprocal*u; V=reciprocal*v; enumFACE |
| edge0 | if d not LT0: vertex0 U0,V0; else if -d LT a: edge0 U=-(d/a),V0; else vertex1 U1,V0 |
| edge1 | U0; if e not LT0: vertex0 V0; else if -e LT c: edge1 V=-(e/c); else vertex2 V1 |
| edge2 | n=((e+c)-b)-d, separate adds/subtracts; if n not GT0: vertex2 U0,V1; else if n LT L: edge2 U=n/L,V=1-U; else vertex1 U1,V0 |

All division is scalar fdivs. The final point is full4
`E2*V + (E1*U + A)` with two source FMAs, after publishing separate outU
then outV. This case table is raw-derived; implementation still requires
independent source review and exceptional-value tests before integration.

All decompiler scalar aliases and VMX aliases (printed operands can differ by
32) must be resolved against the raw graph when implementing; the graph above
does not license copying decompiler pseudo-C. Keep host reciprocal/dot/rsqrt
policy explicit and retain unknown W. No finite-value guard is source-derived.

## Reuse and shortest actual wheel route

Reuse `CollisionTriangleLineInput/Output`, `ReckoningNumeric::dot3`, reciprocal
and reciprocal-square-root estimate services, `cross_full4`, and existing
refinement kernels only where their iteration count/operand graph matches.
Scalar fdivs/fsqrts use the declared host float boundary; do not substitute
reciprocal estimates. Wrapper requires two rsqrt refinements; initial-overlap
normal uses one. Existing typed output retains source failure prefixes.

Integrate under the existing `try_triangle_line` fat gate, retaining thin path
unchanged. `collision_mesh_line.rs` already passes actual request fatness and
zero triangle fatness through every original GP triangle and preserves actual
KD traversal, ordered nearest selection, surface and owner. On successful
wrapper return, this mesh caller **publishes the GP triangle normal**, not the
inner sphere feature normal, while using wrapper-adjusted position. The inner
normal still matters for position correction and must not be skipped.

Use the same actual `SimulationWorldLineQueries` child owners and extend their
lease API to the retained five-test batch described in `skateboard-wheel-line-source.md`.
No substitute capsule raycaster, ground-only shortcut, fake empty membership,
or swallowed unsupported result is needed. DMO remains parked; existing
constructor-empty-owner query semantics and nonempty unpublished refusal remain.

Focused tests: face front/back, edge/vertex tangency, strict overlap vs touching,
parallel/degenerate triangles, region-boundary ties, transition limit, t>1
failure after parameter store, zero/negative/NaN radius gates, wrapper raw-normal
prefix on numeric failure, different one/two-refinement sites, and line-fatness
vs sum-radius position correction. Original deck query needs an independently
checked actual hit/miss in the five-test retained batch and later real
EndWheelLineTest output, before the full pre/postphysics path is claimed.

## Bounded implementation handoff

`crates/skate2-core/src/physics/collision/collision_triangle_line.rs` now dispatches the original
nonzero/unordered sum arm to child `collision_fat_triangle_line.rs`. The child
retains the nearest-point case table, sphere fraction helper, five-transition
region walk and full-vector Option-W arithmetic. Numeric callbacks remain
fallible at their original reduction/estimate sites. The wrapper reports
`UnreadySuccess { field: "position" }` if a native partial success reaches its
position adjustment without any retained position; it never fills that field.
Previously retained result fields remain source scratch rather than being reset.

Nine focused child tests cover front/back offset faces, original-line-radius
correction, overlap/unknown W, rounded edge/vertex results, the independently
corrected absolute-determinant nearest-FACE gate, face failure prefixes,
negative/NaN radius arithmetic, unordered sphere approach, fraction tie/unordered gates, and a scripted
numeric-port five-transition partial-success probe. The latter is explicitly a
branch probe, not claimed real triangle geometry. Existing thin/mesh tests now
check fat-wrapper numeric refusal after the proper normal/GP construction
prefix instead of the removed unsupported-fatness refusal. Root owns Cargo
validation and integration; no World batch or wheel scheduling changed here.

## Trajectory walk consumer (2026-10-23)

`TrajectoryWalkFunctor::operator()` (0x827c1c78) runs the collection
`LineTest<TrajectoryLineTestFunctor>` (0x827c1300). Its mesh body (0x827c01a0)
calls this wrapper at **827c033c** with the trajectory radius as line fatness and
zero triangle fatness, and the functor keeps the wrapper-adjusted position.
`CalculateTimeOfCollision` (**827c1d18**) and the ground-normal box of position
plus and minus the radius (**827c1d8c**) both start from that surface point, not
from the swept-sphere center.

The ground and DMO trajectory sweeps call
`skate2-formats/src/collision.rs::sphere_segment_triangle_intersection`, which now
applies the same correction: the center minus `normal * radius`, using the unit
normal of its face, edge, vertex or overlap case. The line-test path
(`try_triangle_line`) already applied it. The helper's other differences from the
region walk, listed above, remain.

Ledge steps depend on this point. `ConsiderPointVersusEdge` (0x827bfb58) accepts
an edge-moved point only within `testRadius - stepRadius` (0.2 - 0.08) of the
collision position, and the moved point sits `stepRadius * 1.01` (0.0808) behind
the face. From the face contact the distance is 0.0808, so the step onto the top
is accepted. From the sphere center, one trajectory radius (0.05) in front of the
face, it is 0.1308, so the step is rejected.
