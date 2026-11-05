//! Stock authored tree construction; runtime selection remains in the tree.
use super::*;

pub(super) fn build(
    metadata: &AnimationMetadata,
    name: &str,
    construction: &[(AttributeName, AttributeName)],
    parents: &mut Vec<String>,
) -> Result<PlaybackTree, String> {
    let key = name.to_ascii_uppercase();
    if parents.contains(&key) {
        return Err(format!("Cyclic authored animation tree {key}"));
    }
    parents.push(key);
    let result = match metadata.tree(name)? {
        TreeMetadata::Clip(source) => PlaybackTree::Clip {
            name: source.name.clone(),
            clip: PlaybackClip::new(
                f32::from_bits(source.frames_bits),
                f32::from_bits(source.fps_bits),
                f32::from_bits(source.base_speed_bits),
                source.flags_word,
                source
                    .attributes
                    .iter()
                    .map(|a| ClipAttribute {
                        name: encode(a.name.as_bytes()),
                        kind: a.type_id,
                        begin: f32::from_bits(a.begin_bits),
                        end: f32::from_bits(a.end_bits),
                        payload: a.payload_words.clone(),
                    })
                    .collect(),
            ),
        },
        TreeMetadata::BlendSpace(source) => {
            use skate3_core::animation::playback_tree::blend_space::{BlendSpace, Simplex};
            PlaybackTree::BlendSpace(BlendSpace::new(
                source.parameters.iter().map(|p| encode(p.as_bytes())).collect(),
                source.children.iter().map(|child| build(metadata, child, construction, parents)).collect::<Result<_,_>>()?,
                source.simplexes.iter().map(|s| Simplex {
                    children:s.children.clone(),
                    vertices:s.vertex_bits.iter().map(|v| v.iter().map(|&v| f32::from_bits(v)).collect()).collect(),
                    normals:s.normal_bits.iter().map(|v| v.iter().map(|&v| f32::from_bits(v)).collect()).collect(),
                    scales:s.scale_bits.iter().map(|&v| f32::from_bits(v)).collect(),
                }).collect(),
            )?)
        }
        TreeMetadata::PhaseBlend(source) => PlaybackTree::PhaseBlend(PhaseBlend::new(
            encode(source.parameter.as_bytes()),
            source
                .children
                .iter()
                .map(|child| build(metadata, child, construction, parents))
                .collect::<Result<_, _>>()?,
        )?),
        TreeMetadata::SelectionSpace(source) => {
            use skate3_core::animation::playback_tree::selection_space::{
                Candidate, Parameter, SelectionSpace,
            };
            let parameters = source
                .parameters
                .iter()
                .map(|p| Parameter {
                    name: encode(p.name.as_bytes()),
                    mode: p.mode,
                    weight: f32::from_bits(p.weight_bits),
                    minimum: f32::from_bits(p.minimum_bits),
                    maximum: f32::from_bits(p.maximum_bits),
                })
                .collect();
            let candidates = source
                .candidates
                .iter()
                .map(|c| {
                    Ok(Candidate {
                        name: c.child.clone(),
                        values: c.value_bits.iter().map(|v| f32::from_bits(*v)).collect(),
                        tree: build(metadata, &c.child, construction, parents)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            PlaybackTree::SelectionSpace(SelectionSpace::new(parameters, candidates)?)
        }
        //82D1B9B4..BAD8 resolves the first matching construction value at
        //creation. A missing key/value selects child0; this is authored data.
        TreeMetadata::Selector(source) => {
            let value = construction
                .iter()
                .find(|(name, _)| *name == encode(source.parameter.as_bytes()))
                .map(|(_, value)| *value);
            let child = source
                .values
                .iter()
                .position(|key| Some(encode(key.as_bytes())) == value)
                .map(|index| source.children[index].as_str())
                .unwrap_or(&source.default);
            build(metadata, child, construction, parents)?
        }
    };
    parents.pop();
    Ok(result)
}
