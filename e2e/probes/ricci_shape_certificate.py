"""RF2C independent mesh and provenance gates."""
import numpy as np
from ricci_shape_regular_mesh import RESOLUTIONS
from ricci_lattice_mesh import mesh as old_mesh,validate,deck_check
from ricci_lattice_mesh_quality import quality


def certify(m, level):
    required={'vertices','tetrahedra','edges','faces','affine_vertices','node_keys','denominator','subdivision_n'}
    if not required<=m.keys():raise ValueError('missing edgewise identity evidence')
    n=RESOLUTIONS[level];keys=m['node_keys'];ids=keys[:,:4];weights=keys[:,4:]
    if m['denominator']!=12*n or m['subdivision_n']!=n:raise ValueError('wrong fixed resolution')
    if len(m['tetrahedra'])!=14400*n**3:raise ValueError('incomplete child complex')
    if (len(keys)!=len(m['vertices']) or len(np.unique(keys,axis=0))!=len(keys)
        or np.any(weights<0) or np.any(weights.sum(axis=1)!=12*n)
        or np.any((ids<0)!=(weights==0)) or np.any(ids>=120)
        or np.any(ids[:,:-1]>ids[:,1:])):raise ValueError('invalid exact node identity')
    expected=np.einsum('vki,vk->vi',old_mesh(0)['vertices'][np.maximum(ids,0)],weights)/(12*n)
    if not np.allclose(m['affine_vertices'],expected,rtol=0,atol=2e-15):raise ValueError('affine node identity mismatch')
    expected/=np.linalg.norm(expected,axis=1,keepdims=True)
    if not np.allclose(m['vertices'],expected,rtol=0,atol=2e-15):raise ValueError('radial node identity mismatch')
    geometry=validate(m);shape=quality(m['vertices'],m['tetrahedra'])
    if shape['max_diameter_over_inradius']>14:raise ValueError('shape cap exceeded')
    deck=deck_check(m)
    return dict(geometry,**shape,**deck,subdivision_n=n,exact_node_identity_checked=True)
