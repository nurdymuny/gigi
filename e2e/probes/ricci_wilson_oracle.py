"""RF2A independent Koszul/matrix/action oracles and distinct flow experiments."""
import itertools
import numpy as np
import sympy as s
from scipy.integrate import solve_ivp
from ricci_oracle import BRACKET, curvature, velocity


def connection_matrices(g):
    g=np.asarray(g)
    gamma=np.zeros((3,3,3),dtype=np.result_type(g,float))
    for i,j,k in itertools.product(range(3),repeat=3):
        gamma[i,j,k]=(BRACKET[i,j,k]*g[k]-BRACKET[j,k,i]*g[i]+BRACKET[k,i,j]*g[j])/(2*g[k])
    omega=np.empty_like(gamma)
    for i,j,k in itertools.product(range(3),repeat=3):
        omega[i,k,j]=gamma[i,j,k]*np.sqrt(g[k]/g[j])
    return omega


def axial(matrices):
    return np.array([[m[2,1],m[0,2],m[1,0]] for m in matrices])


def phi(g):
    return np.diag(axial(connection_matrices(g)))


def field_curvature(a):
    return np.array([np.cross(a[j],a[k])-2*a[i]
                     for i,j,k in [(0,1,2),(1,2,0),(2,0,1)]])


def energy(g,a):
    f=field_curvature(a)
    return sum(np.dot(f[i],f[i])/(g[(i+1)%3]*g[(i+2)%3]) for i in range(3))/2


def gradient_matrix(g,a):
    """Nine independently differentiated energy coordinates, with L2 weights."""
    out=np.zeros((3,3))
    for i,j in itertools.product(range(3),repeat=2):
        probe=np.array(a,dtype=complex)
        probe[i,j]+=1e-25j
        out[i,j]=-g[i]*energy(g,probe).imag/1e-25
    return out


def wilson(g,x):
    return np.diag(gradient_matrix(np.asarray(g),np.diag(x)))


def mapped_velocity(g,normalized=False):
    g=np.asarray(g,dtype=float)
    v=velocity(g,normalized)
    return phi(g.astype(complex)+1e-25j*v).imag/1e-25


def check_geometry(g):
    g=np.asarray(g)
    matrices=connection_matrices(g)
    np.testing.assert_allclose(matrices+matrices.transpose(0,2,1),0,atol=2e-13)
    solder=np.diag(np.sqrt(g))
    for i,j in itertools.combinations(range(3),2):
        torsion=matrices[i]@solder[j]-matrices[j]@solder[i]-sum(BRACKET[i,j,k]*solder[k] for k in range(3))
        np.testing.assert_allclose(torsion,0,atol=2e-13)
    a=axial(matrices)
    f=field_curvature(a)
    _,_,sectional=curvature(g)
    # F(e_j,e_k) on the orthonormal internal frame has axial coefficient f_i.
    expected=np.array([-sectional[1]*np.sqrt(g[1]*g[2]),
                        -sectional[2]*np.sqrt(g[2]*g[0]),
                        -sectional[0]*np.sqrt(g[0]*g[1])])
    np.testing.assert_allclose(np.diag(f),expected,atol=2e-12,rtol=2e-13)
    return a


def symbolic_certificate():
    lengths=s.symbols("p q r",positive=True)
    g=[v*v for v in lengths]
    x=s.Matrix([(g[(i+1)%3]+g[(i+2)%3]-g[i])/(lengths[(i+1)%3]*lengths[(i+2)%3]) for i in range(3)])
    v=s.Matrix([-4*(g[i]**2-(g[(i+1)%3]-g[(i+2)%3])**2)/(g[(i+1)%3]*g[(i+2)%3]) for i in range(3)])
    p=x.jacobian(lengths)*s.Matrix([v[i]/(2*lengths[i]) for i in range(3)])
    y=s.symbols("x y z",real=True)
    f=[y[(i+1)%3]*y[(i+2)%3]-2*y[i] for i in range(3)]
    e=sum(f[i]**2/(g[(i+1)%3]*g[(i+2)%3]) for i in range(3))/2
    w=s.Matrix([-g[i]*s.diff(e,y[i]) for i in range(3)])
    residual=[s.factor(s.together(p[i]-w[i].subs(dict(zip(y,x))))) for i in range(3)]
    if residual != [0,0,0]: raise AssertionError(f"symbolic H1 residual: {residual}")
    scale=x.jacobian(lengths)*s.Matrix([a/2 for a in lengths])
    if [s.factor(a) for a in scale] != [0,0,0]: raise AssertionError("scale direction")
    acceleration=w.jacobian(lengths)*s.Matrix([v[i]/(2*lengths[i]) for i in range(3)])
    at={lengths[0]:s.sqrt(3),lengths[1]:1,lengths[2]:1,y[0]:-1,y[1]:s.sqrt(3),y[2]:s.sqrt(3)}
    gap=[s.simplify(a.subs(at)) for a in acceleration]
    if gap != [-672,-16*s.sqrt(3),-16*s.sqrt(3)]: raise AssertionError(f"acceleration control {gap}")
    # Check the full nine-coordinate homogeneous action, not only its diagonal restriction.
    entries=s.symbols("a0:9",real=True)
    rows=[s.Matrix(entries[3*i:3*i+3]) for i in range(3)]
    full_f=[rows[(i+1)%3].cross(rows[(i+2)%3])-2*rows[i] for i in range(3)]
    full_energy=sum(full_f[i].dot(full_f[i])/(g[(i+1)%3]*g[(i+2)%3]) for i in range(3))/2
    diagonal={entries[3*i+j]:(y[i] if i==j else 0) for i,j in itertools.product(range(3),repeat=2)}
    off_diagonal=[s.factor(s.diff(full_energy,entries[3*i+j]).subs(diagonal))
                  for i,j in itertools.product(range(3),repeat=2) if i!=j]
    if off_diagonal != [0]*6: raise AssertionError("diagonal ansatz is not invariant")
    return {"h1_residuals":[str(a) for a in residual],"scale_direction":[str(s.factor(a)) for a in scale],
            "off_diagonal_action_derivatives":[str(a) for a in off_diagonal],
            "berger_acceleration_gap":[str(a) for a in gap],"time_scale":1,
            "scope":"positive_diagonal_SU2_symbolic_algebra_not_general_Ricci_theorem"}


def operator_checks():
    rotation=np.array([[0.6,-0.8,0.],[0.8,0.6,0.],[0.,0.,1.]])
    rng=np.random.default_rng(20261117)
    for _ in range(12):
        g=np.exp(rng.uniform(-1,1,3)); a=rng.normal(size=(3,3))
        diagonal_gradient=gradient_matrix(g,np.diag(np.diag(a)))
        np.testing.assert_array_equal(diagonal_gradient-np.diag(np.diag(diagonal_gradient)),0)
        w=gradient_matrix(g,a)
        rotated=a@rotation.T
        np.testing.assert_allclose(gradient_matrix(g,rotated),w@rotation.T,atol=1e-11,rtol=1e-12)
        np.testing.assert_allclose(energy(g,rotated),energy(g,a),atol=1e-11,rtol=1e-12)
        h=rng.normal(size=(3,3))
        finite=(energy(g,a+1e-6*h)-energy(g,a-1e-6*h))/2e-6
        expected=-np.sum(w*h/g[:,None])
        np.testing.assert_allclose(finite,expected,rtol=2e-8,atol=2e-8)
        assert energy(g,a+1e-7*w)<energy(g,a)
    # The map derivative is checked with real finite differences at three resolutions.
    g=np.array([3.,1.,1.]); v=velocity(g,False)
    expected=np.array([48.,-8*np.sqrt(3),-8*np.sqrt(3)])
    errors=[]
    for h in [1e-4,5e-5,2.5e-5]:
        estimate=(phi(g+h*v)-phi(g-h*v))/(2*h)
        errors.append(float(np.max(np.abs(estimate-expected))))
    ratios=[errors[i]/errors[i+1] for i in range(2)]
    assert all(3.8<r<4.2 for r in ratios),ratios
    return {"arbitrary_matrix_gradient_gauge_cases":12,"map_derivative_refinement_ratios":ratios}


def experiment(g0,time,normalized=False,rtol=2e-11):
    """Two independent connection integrations; only H2 receives g(t)."""
    g0=np.asarray(g0,dtype=float)
    rf=solve_ivp(lambda t,g:velocity(g,normalized),(0,time),g0,method="DOP853",
                 dense_output=True,rtol=rtol,atol=rtol/100)
    if not rf.success: raise AssertionError("Ricci reference failed")
    times=np.linspace(0,time,65)
    x0=phi(g0)
    changing=solve_ivp(lambda t,x:wilson(rf.sol(t),x),(0,time),x0,method="DOP853",t_eval=times,rtol=rtol,atol=rtol/100)
    fixed=solve_ivp(lambda t,x:wilson(g0,x),(0,time),x0,method="DOP853",t_eval=times,rtol=rtol,atol=rtol/100)
    if not changing.success or not fixed.success: raise AssertionError("Yang-Mills reference failed")
    mapped=np.array([phi(g) for g in rf.sol(times).T])
    return {"times":times.tolist(),"ricci_metric":rf.sol(times).T.tolist(),
            "mapped_ricci":mapped.tolist(),"moving_background":changing.y.T.tolist(),
            "fixed_background":fixed.y.T.tolist(),
            "moving_max_error":float(np.max(np.abs(mapped-changing.y.T))),
            "fixed_max_gap":float(np.max(np.abs(mapped-fixed.y.T))),
            "fixed_end_gap":(mapped[-1]-fixed.y[:,-1]).tolist(),"rtol":rtol}


def acceleration_check():
    """Centered time differences: the H3 failure starts at second order."""
    g = np.array([3.,1.,1.])
    expected = np.array([-672.,-16*np.sqrt(3),-16*np.sqrt(3)])
    estimates = []
    errors = []
    for h in [4e-4,2e-4,1e-4]:
        gaps = []
        for t in [h,-h]:
            rf = solve_ivp(lambda t,y:velocity(y,False),(0,t),g,method="DOP853",rtol=3e-14,atol=1e-15)
            wf = solve_ivp(lambda t,y:wilson(g,y),(0,t),phi(g),method="DOP853",rtol=3e-14,atol=1e-15)
            if not rf.success or not wf.success:
                raise AssertionError("acceleration reference integration failed")
            gaps.append(phi(rf.y[:,-1])-wf.y[:,-1])
        estimate = (gaps[0]+gaps[1])/h**2
        estimates.append(estimate.tolist())
        errors.append(float(np.max(np.abs(estimate-expected))))
    ratios = [errors[i]/errors[i+1] for i in range(2)]
    assert all(3.8<r<4.2 for r in ratios), ratios
    np.testing.assert_allclose(estimates[-1],expected,rtol=2e-5,atol=.001)
    return {"step_sizes":[4e-4,2e-4,1e-4],"estimates":estimates,"errors":errors,"refinement_ratios":ratios}


def quotient_checks():
    """Integrate invariant densities on the cover and left-translation quotient."""
    result = []
    for g in [np.array([3.,1.,1.]),np.array([.5,1.,2.]),np.array([5.,1.,1.])]:
        a = axial(connection_matrices(g))
        cover_volume = 2*np.pi**2*np.sqrt(np.prod(g))
        cover_energy = cover_volume*energy(g,a)
        cover_mass = cover_volume/g
        volume = cover_volume/120
        quotient_energy = volume*energy(g,a)
        quotient_mass = volume/g
        np.testing.assert_allclose(quotient_energy/cover_energy,1/120,rtol=1e-14)
        np.testing.assert_allclose(quotient_mass/cover_mass,1/120,rtol=1e-14)
        derivative = np.zeros((3,3))
        for i,j in itertools.product(range(3),repeat=2):
            probe = a.astype(complex)
            probe[i,j] += 1e-25j
            derivative[i,j] = (volume*energy(g,probe)).imag/1e-25
        quotient_gradient = -derivative/quotient_mass[:,None]
        np.testing.assert_allclose(quotient_gradient,gradient_matrix(g,a),rtol=1e-13,atol=1e-13)
        result.append({"metric":g.tolist(),"deck_order":120,"cover_volume":float(cover_volume),
            "quotient_volume":float(volume),"cover_energy":float(cover_energy),
            "quotient_energy":float(quotient_energy),"quotient_mass":quotient_mass.tolist(),
            "quotient_gradient":quotient_gradient.tolist()})
    return result


if __name__=="__main__":
    print(symbolic_certificate())
    print(operator_checks())
    for g in [[3.,1.,1.],[.5,1.,2.],[5.,1.,1.]]:
        r=experiment(g,.02)
        print(g, r["moving_max_error"],r["fixed_max_gap"],r["fixed_end_gap"])
